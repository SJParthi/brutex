//! Gate 8 for `crates/vocab` — the mask operations cost the same whatever the
//! masks contain, because the sweep's inner loop is one of them.
//!
//! # Why this crate needs a bench at all
//!
//! `ConditionMask::hits` is the hot path of the entire engine. A sweep evaluates
//! it once per candidate per bar, so at 1.2 million bars and a frontier of any
//! interesting size it is called more often than everything else in this
//! repository put together. `CLAUDE.md` §3 rule 4 names mask evaluation as one
//! of the five operations that must be constant, and until this file existed
//! that claim was made by a comment and checked by nothing.
//!
//! # What is measured, and why a ratio rather than a number
//!
//! An absolute nanosecond count on shared CI measures the runner. What survives
//! moving between machines is the RATIO between two inputs that must cost the
//! same. So every measurement here is a pair:
//!
//! * a candidate that **hits** against one that **misses**, because a branchless
//!   test must not be faster when the answer is no;
//! * a miss in **word 0** against a miss in **word 5**, because an early-exit
//!   loop would return sooner on the first and that is precisely the
//!   implementation `hits` refuses to be;
//! * a **1-bit** candidate against a **234-bit** one, because the cost must not
//!   depend on how much the candidate requires — that is what lets the Apriori
//!   ladder walk to any depth without the per-test cost growing with `k`.
//!
//! The third pair is the one that matters for §6. If evaluating a k=12
//! combination cost more per bar than a k=1 one, the ladder's cost would be
//! superlinear in depth and "no depth parameter" would be unaffordable rather
//! than principled.
//!
//! # What this bench cannot see
//!
//! It measures the compiled `hits` on this machine's architecture. It does not
//! prove branchlessness — a compiler is free to introduce a branch, and a ratio
//! near 1.0 is evidence rather than proof. The source-level guarantee is
//! `vocab::mask::hits_does_the_same_work_for_every_input`, which reads the
//! function body and refuses a loop, an early return and a word it does not
//! touch. The two together are the claim: the source has no branch to take, and
//! the clock agrees it does not take one.
//!
//! See `crates/store/benches/ratio.rs` for why there is no benchmarking
//! framework and why every ratio here is integer arithmetic.

use std::hint::black_box;
use std::time::Instant;

use vocab::ConditionMask;
use vocab::mask::WORDS;
use vocab::table::{LIVE, NEXT_FREE};

/// A ratio above this is a failure. Thousandths, so the comparison is integer.
///
/// 3.0x — `docs/04-invariants.md`, the shared-CI number every other crate's
/// bench uses. A branchless operation should land near 1.0x; the ceiling is
/// this loose because a shared runner's scheduler is noisier than the thing
/// being measured.
const CEILING_PERMILLE: u128 = 3_000;

/// How many times each measurement is repeated before the minimum is taken.
///
/// The minimum, not the mean: a scheduler can only ever make a measurement
/// slower, so the smallest observation is the closest one to the real cost.
const TRIALS: u32 = 40;

/// Times one closure, in picoseconds per call, as a minimum over [`TRIALS`].
fn cost_ps<T>(reps: u32, mut op: impl FnMut() -> T) -> u128 {
    let mut best = u128::MAX;
    for _ in 0..TRIALS {
        let start = Instant::now();
        for _ in 0..reps {
            black_box(op());
        }
        let ps = start.elapsed().as_nanos() * 1_000 / u128::from(reps);
        if ps < best {
            best = ps;
        }
    }
    best
}

/// Prints one measurement and returns whether it stayed under the ceiling.
///
/// A zero baseline is a FAILURE and not a pass. An operation that timed at zero
/// picoseconds was optimised away, and a ratio against nothing is not a
/// measurement — reporting it as ok is the fallback `CLAUDE.md` §4 bans.
fn ratio(label: &str, base_ps: u128, at_ps: u128) -> bool {
    if base_ps == 0 {
        println!("  {label:<58} UNMEASURABLE — the 1x baseline timed at zero");
        return false;
    }
    let permille = at_ps * 1_000 / base_ps;
    let ok = permille <= CEILING_PERMILLE;
    println!(
        "  {label:<58} {:>8} ps -> {:>8} ps   ratio {}.{:03}x  {}",
        base_ps,
        at_ps,
        permille / 1_000,
        permille % 1_000,
        if ok { "ok" } else { "BREACH" }
    );
    ok
}

/// A bar carrying every live bit. The most permissive bar there is: every
/// candidate drawn from `LIVE` hits it.
fn every_live_bit() -> ConditionMask {
    LIVE
}

/// A candidate requiring exactly one bit, in the given word.
fn one_bit_in_word(word: usize) -> ConditionMask {
    let bit = u32::try_from(word).unwrap_or(0) * 64;
    ConditionMask::ZERO.with_bit(bit)
}

/// A candidate requiring every live bit — the widest one the table permits.
fn all_live_bits() -> ConditionMask {
    LIVE
}

/// C-V-01 — the answer's VALUE does not change the answer's COST.
///
/// A hit and a miss must cost the same. If a miss were cheaper the engine's
/// per-bar cost would depend on how selective the frontier is, and the sweep's
/// runtime would vary with the data rather than with its size.
fn a_hit_and_a_miss_cost_the_same() -> bool {
    let bar = every_live_bit();
    let empty = ConditionMask::ZERO;

    // Hits: the empty candidate requires nothing, so it hits every bar.
    let hit = cost_ps(200_000, || black_box(&bar).hits(black_box(&empty)));

    // Misses: a candidate requiring a bit past the live set. `NEXT_FREE` is
    // allocated to nothing, so no bar can carry it and this always fails.
    let unreachable = ConditionMask::ZERO.with_bit(u32::from(NEXT_FREE));
    let miss = cost_ps(200_000, || black_box(&bar).hits(black_box(&unreachable)));

    ratio("C-V-01 hits: a HIT -> a MISS", hit, miss)
}

/// C-V-02 — WHERE the miss happens does not change the cost.
///
/// This is the measurement that would catch an early-exit loop, which is the
/// implementation `hits` exists to not be. A loop returning on the first
/// non-matching word is fast on a candidate that fails in word 0 and slow on
/// one that fails in word 5, so the sweep's cost would depend on which
/// conditions a combination happens to require — a constant-time guarantee that
/// holds only on average, which §3 rule 4 does not accept.
fn a_miss_costs_the_same_in_every_word() -> bool {
    let bar = ConditionMask::ZERO;
    let first = one_bit_in_word(0);
    let last = one_bit_in_word(WORDS - 1);

    let early = cost_ps(200_000, || black_box(&bar).hits(black_box(&first)));
    let mut ok = true;
    ok &= ratio(
        "C-V-02 hits: miss in word 0 -> miss in the last word",
        early,
        cost_ps(200_000, || black_box(&bar).hits(black_box(&last))),
    );

    // Every word in between, so a partially unrolled loop is caught too.
    for w in 1..WORDS - 1 {
        let mid = one_bit_in_word(w);
        ok &= ratio(
            &format!("C-V-02 hits: miss in word 0 -> miss in word {w}"),
            early,
            cost_ps(200_000, || black_box(&bar).hits(black_box(&mid))),
        );
    }
    ok
}

/// C-V-03 — HOW MUCH the candidate requires does not change the cost.
///
/// The claim §6 rests on. The Apriori ladder walks upward from k=1 with no
/// depth parameter, and that is only affordable if evaluating a k=234
/// combination costs what a k=1 one costs. A per-bit cost would make the
/// ladder's total work quadratic in depth and the absent parameter would be a
/// performance bug rather than a design decision.
fn the_cost_does_not_grow_with_what_the_candidate_requires() -> bool {
    let bar = every_live_bit();
    let one = one_bit_in_word(0);
    let all = all_live_bits();

    let narrow = cost_ps(200_000, || black_box(&bar).hits(black_box(&one)));
    let wide = cost_ps(200_000, || black_box(&bar).hits(black_box(&all)));

    println!(
        "  {:<58} {} bit(s) -> {} bit(s)",
        "C-V-03 candidate width (context)",
        one.popcount(),
        all.popcount()
    );
    ratio(
        "C-V-03 hits: a 1-bit candidate -> every live bit",
        narrow,
        wide,
    )
}

/// C-V-04 — the set operations are the same shape as `hits`.
///
/// `union`, `intersect` and `popcount` are each `WORDS` word operations with no
/// loop over set bits. `popcount` is the one worth measuring: a naive
/// implementation counts bits by shifting, which costs more when more are set,
/// and the reconciliation the engine's frontier does calls it on every mask.
fn the_set_operations_do_not_grow_with_the_bits_set() -> bool {
    let empty = ConditionMask::ZERO;
    let full = all_live_bits();
    let one = one_bit_in_word(0);

    let mut ok = true;

    let sparse = cost_ps(200_000, || black_box(&one).popcount());
    ok &= ratio(
        "C-V-04 popcount: 1 bit set -> every live bit set",
        sparse,
        cost_ps(200_000, || black_box(&full).popcount()),
    );

    let union_empty = cost_ps(200_000, || black_box(&empty).union(black_box(&one)));
    ok &= ratio(
        "C-V-04 union: empty|one -> full|full",
        union_empty,
        cost_ps(200_000, || black_box(&full).union(black_box(&full))),
    );

    let intersect_empty = cost_ps(200_000, || black_box(&empty).intersect(black_box(&one)));
    ok &= ratio(
        "C-V-04 intersect: empty&one -> full&full",
        intersect_empty,
        cost_ps(200_000, || black_box(&full).intersect(black_box(&full))),
    );

    ok
}

fn main() {
    println!("gate 8 — crates/vocab, ceiling {CEILING_PERMILLE} permille");
    println!(
        "  mask is {WORDS} words = {} bits, {} allocated, {} live",
        ConditionMask::BITS,
        NEXT_FREE,
        LIVE.popcount()
    );
    let mut ok = true;
    ok &= a_hit_and_a_miss_cost_the_same();
    ok &= a_miss_costs_the_same_in_every_word();
    ok &= the_cost_does_not_grow_with_what_the_candidate_requires();
    ok &= the_set_operations_do_not_grow_with_the_bits_set();
    if ok {
        println!("all ratios within the ceiling");
    } else {
        println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
        std::process::exit(1);
    }
}
