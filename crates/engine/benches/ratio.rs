//! Gate 8 for `crates/engine` — the per-bar cost of a sweep does not depend on
//! how many bars there are, how much a combination requires, or what the answer
//! is.
//!
//! # The three things a sweep is allowed to cost, and the one it is not
//!
//! `CLAUDE.md` §3 rule 4 names mask evaluation among the five operations that
//! must be O(1). Support counting is **O(bars)** and that is not a violation: it
//! *is* the measurement, and there is no way to count how many bars match without
//! looking at each one. The Apriori level join is **O(|frontier|²)**, which is
//! that algorithm's documented shape. Both are stated in
//! `docs/10-shared-core.md` rather than hidden.
//!
//! What must not happen is a **per-bar cost that grows**. If the constant factor
//! in front of O(bars) drifted with the column length, with the candidate's
//! width, or with how many bars match, then:
//!
//! * the sweep's runtime would be superlinear in the data, and the 1.22 million
//!   bars an instrument carries would cost more than 122 times what 10,000 do;
//!   and
//! * §6's absent depth parameter would become unaffordable rather than
//!   principled, because evaluating a k=12 combination would cost more per bar
//!   than a k=1 one and the ladder's total work would grow quadratically in
//!   depth.
//!
//! So every row here divides by the bar count and compares the quotient. An
//! absolute nanosecond figure measures the runner; cost per bar is what survives
//! moving between machines.
//!
//! # Why the answer's VALUE gets its own row
//!
//! `support` folds `ConditionMask::hits` over the column with no early exit, and
//! `hits` is branchless. A version that stopped early once the answer was
//! decided, or that skipped work on a miss, would be faster on selective
//! candidates — and the sweep's runtime would then depend on the market rather
//! than on the number of bars. A bound measured on data where everything matches
//! would be wrong on data where nothing does.
//!
//! # What this bench cannot see
//!
//! It measures synthetic bar columns, so it says nothing about whether the bits
//! are the bits real data would carry — the emission guarantees live in
//! `crates/indicators`. It cannot see a cost that appears only past 1,000,000
//! bars, which is the deepest column here against the 1,222,791 an instrument
//! actually holds; that gap is stated rather than implied, per §3 rule 6. And it
//! measures the ladder's *per-bar* factor, not the O(|frontier|²) join, which is
//! not a per-bar cost and is not claimed to be constant.
//!
//! See `crates/store/benches/ratio.rs` for why there is no benchmarking
//! framework and why every ratio here is integer arithmetic.

use std::hint::black_box;
use std::time::Instant;

use engine::column::{Column, SUPPORT_BLOCK_ROWS, support_block};
use engine::{Itemset, Ladder, support};
use vocab::ConditionMask;

/// A ratio above this is a failure. Thousandths, so the comparison is integer.
///
/// 3.0x — `docs/04-invariants.md`, the shared-CI number.
const CEILING_PERMILLE: u128 = 3_000;

/// How many times each measurement is repeated before the minimum is taken.
///
/// Lower than the other benches because each repetition walks a whole bar
/// column; the minimum of eight passes over a million masks is already a stable
/// number, and forty would spend minutes to move the last digit.
const TRIALS: u32 = 8;

/// The bar columns measured, shortest first. The first is the baseline.
const COLUMNS: [usize; 3] = [10_000, 100_000, 1_000_000];

/// Positions the synthetic bars draw from. Well inside the live table, and
/// spread across words so no measurement is confined to word 0.
const DRAWN_FROM: [u32; 8] = [3, 17, 70, 101, 150, 199, 210, 233];

/// Times one closure once per trial, in picoseconds, as a minimum.
///
/// Unlike the other benches this does not loop inside the timer: one call here
/// already walks an entire column, so the per-call cost is large and the loop
/// would only multiply it.
fn once_ps<T>(mut op: impl FnMut() -> T) -> u128 {
    let mut best = u128::MAX;
    for _ in 0..TRIALS {
        let start = Instant::now();
        black_box(op());
        let ps = start.elapsed().as_nanos() * 1_000;
        if ps < best {
            best = ps;
        }
    }
    best
}

/// Prints one measurement and returns whether it stayed under the ceiling.
///
/// Both directions breach. For a per-bar cost, a variant that is CHEAPER is as
/// much a data dependence as one that is dearer — a `support` that skipped work
/// when nothing matched would look excellent one-sided and would make the
/// sweep's runtime a function of the market.
///
/// A zero baseline FAILS rather than dividing by zero: an operation the optimiser
/// deleted has not been shown constant, it has been shown absent, and reporting
/// that as a pass is the fallback `CLAUDE.md` §4 bans.
fn ratio(label: &str, base_ps: u128, at_ps: u128) -> bool {
    if base_ps == 0 || at_ps == 0 {
        println!("  {label:<60} UNMEASURABLE — a side timed at zero");
        return false;
    }
    let up = at_ps * 1_000 / base_ps;
    let down = base_ps * 1_000 / at_ps;
    let ok = up.max(down) <= CEILING_PERMILLE;
    println!(
        "  {label:<60} {:>7} ps/bar -> {:>7} ps/bar   ratio {}.{:03}x  {} {}",
        base_ps,
        at_ps,
        up / 1_000,
        up % 1_000,
        if ok { "ok" } else { "BREACH" },
        if down > up { "(cheaper)" } else { "" },
    );
    ok
}

/// Prints a one-sided per-unit growth measurement.
///
/// Unlike [`ratio`], a cheaper larger input is allowed. This is only correct
/// for a composite operation with fixed work outside the input-sized pass:
/// `T(n) = a*n + b`, so `T(n)/n = a + b/n` legitimately falls as `n` grows.
/// The caller must first prove both inputs performed the same non-vacuous
/// structural work; otherwise an early exit could buy a dishonest pass.
fn growth_ratio(label: &str, base_ps: u128, at_ps: u128) -> bool {
    if base_ps == 0 || at_ps == 0 {
        println!("  {label:<60} UNMEASURABLE — a side timed at zero");
        return false;
    }
    let up = at_ps * 1_000 / base_ps;
    let ok = up <= CEILING_PERMILLE;
    println!(
        "  {label:<60} {:>7} ps/bar -> {:>7} ps/bar   ratio {}.{:03}x  {} {}",
        base_ps,
        at_ps,
        up / 1_000,
        up % 1_000,
        if ok { "ok" } else { "BREACH" },
        if at_ps < base_ps {
            "(cheaper; fixed work amortised)"
        } else {
            ""
        },
    );
    ok
}

/// Refuses loudly rather than unwrapping into a panic message nobody reads.
/// The per-bar floor: the cheapest possible walk over the same column.
///
/// # Why this exists
///
/// Every ratio in this file divides one per-bar cost by another per-bar cost of
/// the SAME operation -- column length against column length, depth against
/// depth, hit against miss. An independent audit measured what that cannot see: a
/// mask operation 174x slower passed its crate's ratio rows at 0.98x-1.00x,
/// because a uniform cost cancels in a quotient. `support` calls `hits` once per
/// bar, so exactly that regression would leave every row here reporting ok.
///
/// The denominator has to be something that cannot move when `support` does. This
/// walks the same slice, touching each bar, doing one `wrapping_add` instead of
/// one `hits`. It carries the same loop, the same bounds checks and the same
/// memory traffic, so the quotient is "how many of the cheapest per-bar things
/// does a supported bar cost" -- and it scales with the runner without scaling
/// with a regression.
fn floor_ps_per_bar(bars: &[ConditionMask]) -> u128 {
    let n = u128::try_from(bars.len()).unwrap_or(1).max(1);
    let total = once_ps(|| {
        let mut acc = 0u64;
        for b in black_box(bars) {
            acc = acc.wrapping_add(black_box(b.words()[0]));
        }
        black_box(acc)
    });
    total / n
}

/// Prints one budget in floors and returns whether it held.
///
/// A breached RATIO says the cost depends on the data. A breached BUDGET says the
/// cost rose for every input at once, which no ratio in this file can report.
fn budget(label: &str, floor: u128, at_ps: u128, allowed: u128) -> bool {
    if floor == 0 {
        println!("  {label:<58} UNMEASURABLE — the floor timed at zero");
        return false;
    }
    let floors = at_ps * 1_000 / floor;
    let ok = floors <= allowed * 1_000;
    println!(
        "  {label:<58} {:>8} ps/bar = {}.{:03} floors, budget {allowed}   {}",
        at_ps,
        floors / 1_000,
        floors % 1_000,
        if ok { "ok" } else { "OVER BUDGET" }
    );
    ok
}

/// C-E-05 — one supported bar costs a bounded multiple of the per-bar floor.
fn support_stays_within_its_budget() -> bool {
    /// Floors allowed per bar.
    ///
    /// Measured, arm64 laptop, release, `lto = "fat"`: **1.287 floors** at a floor
    /// of 1202 ps/bar, and **2.031** at a floor of 417 ps/bar on the next run --
    /// k=1 and k=8 agreeing to three digits within each run. That spread is
    /// reported rather than averaged: `once_ps` takes 8 trials, both legs walk 100
    /// 000 masks, and the floor is the noisier of the two because it does less work
    /// per unit of memory traffic. The budget is sized on the WORST observed
    /// multiple, not the mean. The multiple is near one because
    /// both legs walk the same 100_000-mask slice and are memory-bound rather than
    /// arithmetic-bound -- which is the point: the floor carries the same traffic
    /// `support` does, so what remains in the quotient is the `hits` call itself.
    ///
    /// Six leaves 4.6x for a different microarchitecture and refuses the 174x
    /// regression that every ratio in this file reports as ok by a factor of 37.
    const ALLOWED: u128 = 6;

    let bars = column(100_000);
    let floor = floor_ps_per_bar(&bars);
    let owned = Column::from_rows(&bars);
    println!("  the per-bar floor is {floor} ps — one black-boxed wrapping_add per bar");
    let mut ok = true;
    for k in [1usize, DRAWN_FROM.len()] {
        ok &= budget(
            &format!("C-E-05 support at k={k}"),
            floor,
            support_ps_per_bar(&owned, &candidate(k)),
            ALLOWED,
        );
    }
    ok
}

fn refuse(what: &str) -> ! {
    println!("BENCH SETUP FAILED — {what}");
    std::process::exit(1)
}

/// A synthetic bar column of `n` bars, each carrying a deterministic subset of
/// [`DRAWN_FROM`].
///
/// Deterministic — no clock, no randomness — so two runs measure the same work.
/// The subset is chosen by the bar's index against a coprime stride, so
/// consecutive bars differ and no short cycle repeats.
fn column(n: usize) -> Vec<ConditionMask> {
    (0..n)
        .map(|i| {
            let mut m = ConditionMask::ZERO;
            for (slot, bit) in DRAWN_FROM.iter().enumerate() {
                // A different subset per bar, spread over the drawn positions.
                if (i / (slot + 1) + i * 7) % 3 != 0 {
                    m = m.with_bit(*bit);
                }
            }
            m
        })
        .collect()
}

/// The owned column the live ladder evaluates.
fn owned_column(n: usize) -> Column {
    Column::from_rows(&column(n))
}

/// A column where every bar carries every drawn position, so any candidate drawn
/// from them matches every bar.
fn column_all_set(n: usize) -> Vec<ConditionMask> {
    let mut full = ConditionMask::ZERO;
    for bit in DRAWN_FROM {
        full = full.with_bit(bit);
    }
    vec![full; n]
}

/// A candidate requiring the first `k` of [`DRAWN_FROM`].
fn candidate(k: usize) -> ConditionMask {
    let mut m = ConditionMask::ZERO;
    for bit in DRAWN_FROM.iter().take(k) {
        m = m.with_bit(*bit);
    }
    m
}

/// Cost per bar, in picoseconds, of the support method the live ladder calls,
/// [`Column::support_each`], asked of ONE candidate.
///
/// Until D-4481 this timed [`Column::support`], which the walk called once per
/// candidate. The walk now counts each lane's whole slice through
/// `support_each`, so the rows that use this helper (C-E-02, C-E-03, C-E-05,
/// C-E-06, C-E-09) time the function that ships, at the one-candidate end of
/// its range, where it does the most memory traffic per hit test.
fn support_ps_per_bar(column: &Column, mask: &ConditionMask) -> u128 {
    let n = u128::from(column.bars()).max(1);
    let mut one = [Itemset {
        mask: *mask,
        hits: 0,
    }];
    once_ps(|| {
        column.support_each(black_box(&mut one));
        one.first().map_or(0, |item| item.hits)
    }) / n
}

/// How many candidates the batch rows count at once.
///
/// A lane of `drain` holds up to `BATCH_PER_LANE` (8,192) candidates and k=1
/// holds up to 384; 64 is far below either, so the row does not flatter the
/// batch by making it larger than a small level's lane would be.
const BATCH_CANDIDATES: usize = 64;

/// [`BATCH_CANDIDATES`] distinct candidates over [`DRAWN_FROM`]: candidate `i`
/// requires the drawn positions named by the set bits of `i + 1`, so every one
/// is a different non-empty subset and most of them match some bars.
fn batch_candidates() -> Vec<Itemset> {
    (1..=BATCH_CANDIDATES)
        .map(|subset| Itemset {
            mask: DRAWN_FROM
                .iter()
                .enumerate()
                .filter(|(slot, _)| subset >> slot & 1 == 1)
                .fold(ConditionMask::ZERO, |m, (_, &bit)| m.with_bit(bit)),
            hits: 0,
        })
        .collect()
}

/// C-E-01 — the per-bar cost of support counting does not grow with the column.
///
/// Support counting is O(bars) by definition. This row is about the CONSTANT in
/// front of it: 1,000,000 bars must cost 100 times what 10,000 do, not more. A
/// drifting constant is how an O(bars) measurement becomes superlinear without
/// anything in the source looking like a nested loop.
///
/// # What it times since D-4481, and why the old number was not the answer
///
/// This timed ONE candidate through `Column::support`, the way the walk used
/// to count, and printed 0.663x for the million-bar leg once -- while the same
/// call measured 1.2 ns per bar at 10^4 and 4.4 to 18 ns at 10^6 on a loaded
/// four-vCPU box (so1-1). One candidate per pass streams the whole column from
/// memory for six word-ANDs a bar, so its per-bar cost is the memory system's,
/// and the minimum of eight trials catches whichever trial the cache happened
/// to favour. The walk now counts a lane's whole slice in one blocked pass, and
/// this row walks the column exactly as [`Column::support_each`] does -- every
/// full block in order, each counted for [`BATCH_CANDIDATES`] candidates by
/// [`support_block`] -- timing each block, `TRIALS` passes with the three
/// columns taken in turn, and compares the MEDIAN block per (bar, candidate)
/// pair. One call over a million bars runs for tens of milliseconds, so on a
/// shared box every trial of it is preempted and the minimum of eight is the
/// scheduler's number; a block is microseconds, and the median of thousands
/// of them is the code's. The one-candidate figure is printed by FXD-03 and
/// named in `docs/06-limits.md`.
fn support_costs_the_same_per_bar_at_every_column_length() -> bool {
    let columns: Vec<Vec<ConditionMask>> = COLUMNS.iter().map(|&n| column(n)).collect();
    let mut items = batch_candidates();
    let mut per_block: Vec<Vec<u128>> = columns
        .iter()
        .map(|rows| Vec::with_capacity(rows.len() / SUPPORT_BLOCK_ROWS * TRIALS as usize))
        .collect();
    for _ in 0..TRIALS {
        for (rows, times) in columns.iter().zip(per_block.iter_mut()) {
            for block in rows.chunks_exact(SUPPORT_BLOCK_ROWS) {
                let start = Instant::now();
                support_block(black_box(block), &mut items);
                times.push(start.elapsed().as_nanos());
            }
        }
    }
    let pairs = u128::try_from(SUPPORT_BLOCK_ROWS * BATCH_CANDIDATES).unwrap_or(1);
    let ps_per_pair: Vec<u128> = per_block
        .iter_mut()
        .map(|times| {
            times.sort_unstable();
            rank(times, 500) * 1_000 / pairs
        })
        .collect();
    let Some(&base) = ps_per_pair.first() else {
        refuse("COLUMNS is empty, so there is no baseline")
    };
    let mut ok = true;
    for (n, &at) in COLUMNS.iter().zip(&ps_per_pair).skip(1) {
        ok &= ratio(
            &format!(
                "C-E-01 support_each x{BATCH_CANDIDATES}: {} bars -> {n} bars",
                COLUMNS.first().copied().unwrap_or(0)
            ),
            base,
            at,
        );
    }
    ok
}

/// C-E-02 — the per-bar cost does not grow with what the combination requires.
///
/// §6's row. The ladder walks upward from k=1 with no depth parameter and stops
/// where the frequent frontier empties. That is affordable only because a k=8
/// combination costs per bar what a k=1 one costs — `hits` is `WORDS` word
/// operations whatever the popcount. If this drifted, the absent parameter would
/// be a performance defect rather than a design decision.
fn support_costs_the_same_per_bar_at_every_depth() -> bool {
    let column = owned_column(100_000);
    let base = support_ps_per_bar(&column, &candidate(1));

    let mut ok = true;
    for k in [4, DRAWN_FROM.len()] {
        ok &= ratio(
            &format!("C-E-02 support: a k=1 candidate -> a k={k} candidate"),
            base,
            support_ps_per_bar(&column, &candidate(k)),
        );
    }
    ok
}

/// `n` distinct offered positions, pre-sized as production k=1 pre-sizes them
/// and filled through the production [`engine::primitives::offer`].
fn offered_of(n: usize) -> std::collections::HashSet<u32> {
    let mut set = std::collections::HashSet::with_capacity(n);
    let end = u32::try_from(n).unwrap_or(u32::MAX);
    for position in 0..end {
        engine::primitives::offer(&mut set, position);
    }
    set
}

/// C-E-10 — k=1 duplicate rejection costs the same however much was offered.
///
/// # Why this row exists beside C-E-04
///
/// `CLAUDE.md` §3 rule 4 names duplicate rejection among the five operations.
/// Production performs it only at k=1: `offered` is a pre-sized `HashSet<u32>`
/// and `insert` returning false rejects a repeated position. At k≥2 the prefix
/// join is injective and there is no candidate-dedup table to benchmark.
///
/// The insert is [`engine::primitives::offer`], the function
/// `Ladder::first_level` itself calls, not a bench-local `insert` that only
/// resembles it: until D-0924 this row built and probed its own set, so an O(n)
/// regression in the production operation would have left it green.
///
/// So this varies that exact production-shaped table and nothing else: 1,000 /
/// 10,000 / 100,000 already accepted positions, followed by repeated insertion
/// of position zero. Pre-sizing matters: timing an unsized mask set, or a miss
/// that grows it past its declared input width, would measure an implementation
/// the live path does not use. The result is expected/amortised hash-table
/// evidence, not an adversarial worst-case guarantee.
fn duplicate_rejection_costs_the_same_however_much_is_seen() -> bool {
    // `once_ps` times ONE call and a hash probe is nanoseconds, so the repeats
    // go INSIDE and the total is divided by them. Same shape as C-E-11 below.
    //
    // DECLARED FIRST, not beside the closure that reads it: an item exists from
    // the start of its scope whatever line it is written on, so `const` after a
    // `let` reads as sequential when it is not. `clippy::items_after_statements`
    // refuses the spelling for that reason, and it is denied workspace-wide.
    const REPS: usize = 20_000;

    let mut small = offered_of(1_000);
    let mut medium = offered_of(10_000);
    let mut large = offered_of(100_000);

    let probe = |set: &mut std::collections::HashSet<u32>| -> u128 {
        let total = once_ps(|| {
            let mut rejected = 0_usize;
            for _ in 0..REPS {
                if !engine::primitives::offer(black_box(&mut *set), black_box(0)) {
                    rejected = rejected.saturating_add(1);
                }
            }
            black_box(rejected)
        });
        total / u128::try_from(REPS).unwrap_or(1).max(1)
    };

    let mut ok = true;
    let base = probe(&mut small);
    ok &= ratio(
        "C-E-10 k=1 duplicate: 1,000 -> 10,000 offered",
        base,
        probe(&mut medium),
    );
    ok &= ratio(
        "C-E-10 k=1 duplicate: 1,000 -> 100,000 offered",
        base,
        probe(&mut large),
    );
    ok
}

/// C-E-11 — appending one result costs the same however many are held.
///
/// # Why this row exists
///
/// The fifth operation rule 4 names. Production k=1 reserves the offered width;
/// at k≥2 `out` reserves the previous survivor count capped by the candidate
/// ceiling. This row measures the part that reservation actually makes flat:
/// pushes that fit in already allocated storage, using the exact `Itemset`
/// element type production retains.
///
/// The push is [`engine::primitives::append`], the function the batch drain
/// itself calls per frequent candidate (D-0924), not a bench-local `push`.
///
/// The vector is allocated before the timer and cleared between trials. That
/// keeps allocator behavior out of an append measurement and matches a live
/// level after its pre-sizing step. It does **not** prove an individual push is
/// worst-case O(1) after an expanding level outgrows the heuristic; that path
/// retains `Vec`'s amortised O(1) growth bound and may move all held elements.
fn result_append_costs_the_same_however_many_are_held() -> bool {
    let one = Itemset {
        mask: candidate(3),
        hits: 1,
    };
    let per_push = |n: usize| -> u128 {
        let mut out: Vec<Itemset> = Vec::with_capacity(n);
        let total = once_ps(|| {
            out.clear();
            for _ in 0..n {
                engine::primitives::append(&mut out, black_box(one));
            }
            black_box(out.len())
        });
        total / u128::try_from(n).unwrap_or(1).max(1)
    };

    let base = per_push(1_000);
    let mut ok = true;
    ok &= ratio(
        "C-E-11 append: 1,000 -> 10,000 held",
        base,
        per_push(10_000),
    );
    ok &= ratio(
        "C-E-11 append: 1,000 -> 100,000 held",
        base,
        per_push(100_000),
    );
    ok
}

/// The sizes the p99 rows sweep: 10^3 to 10^6 offered positions, held
/// results or bars.
const P99_SIZES: [usize; 4] = [1_000, 10_000, 100_000, 1_000_000];

/// Operations per sample in O1P-03 and O1P-04.
///
/// A probe or a push is a few nanoseconds, below what one `Instant` pair
/// resolves cleanly, so each sample times [`P99_BATCH`] consecutive
/// operations and the distribution is of those batches.
const P99_BATCH: usize = 32;

/// Groups per p99 row, and samples per size inside one group (D-4482).
///
/// # Why groups, and why the gate reads their median
///
/// The rows used to run every round of 10^3, then every round of 10^4, and so
/// on, and to gate the smallest round p99 of each size against the smallest of
/// 10^3. On this shared four-vCPU box that breached O1P-04 with no defect in
/// the code (so1-3): the base was measured in one stretch of the machine's
/// load and the size it was divided into in another, and a two-sided ceiling
/// fails a quiet base as readily as a loud comparand.
///
/// So every group measures every size, back to back, starting at a different
/// size each group, and each group yields its own p99 ratio against its own
/// 10^3. A load spike lands inside one or two groups and moves their ratios;
/// an O(n) operation moves every group's. The row gates the MEDIAN of the
/// nine paired ratios, so it takes five disturbed groups of nine to move the
/// verdict, and `the_interleaved_statistic_can_pass_and_can_fail` proves both
/// halves of that on fixed numbers before any row is trusted.
const P99_GROUPS: usize = 9;
const P99_GROUP_SAMPLES: usize = 5_000;

/// One size's shape inside one group, nanoseconds per sample.
#[derive(Clone, Copy)]
struct Tail {
    p50: u128,
    p99: u128,
    max: u128,
}

/// The sample at `permille` thousandths of a sorted group.
fn rank(sorted: &[u128], permille: usize) -> u128 {
    let at = (sorted.len() * permille / 1_000).min(sorted.len().saturating_sub(1));
    sorted.get(at).copied().unwrap_or(0)
}

/// Times `sample` `samples` times; it is handed a number that differs per call.
///
/// The first `warmup` calls run untimed. Inside a group the sizes run back to
/// back, so a size's tables start the group evicted by the size before it;
/// what the row asks about is the operation on a table it has been using,
/// and a cold start would put the eviction, not the operation, at the p99.
fn group_tail(
    warmup: usize,
    samples: usize,
    salt: u64,
    mut sample: impl FnMut(u64) -> usize,
) -> Tail {
    let mut ns = Vec::with_capacity(samples);
    for at in 0..(warmup + samples) as u64 {
        let start = Instant::now();
        black_box(sample(salt.wrapping_mul(1 << 32) ^ at));
        let took = start.elapsed().as_nanos();
        if at >= warmup as u64 {
            ns.push(took);
        }
    }
    ns.sort_unstable();
    Tail {
        p50: rank(&ns, 500),
        p99: rank(&ns, 990),
        max: ns.last().copied().unwrap_or(0),
    }
}

/// Every size inside every group: `measure(at, n, group)` is the samples of
/// size `n`, which is `sizes[at]`.
///
/// Returns one vector per size, holding that size's [`P99_GROUPS`] tails in
/// group order. Inside a group the sizes run back to back, and the size that
/// goes first rotates, so a load that rises through a group does not always
/// land on the same size.
fn interleaved(
    sizes: &[usize],
    mut measure: impl FnMut(usize, usize, usize) -> Tail,
) -> Vec<Vec<Tail>> {
    let mut out: Vec<Vec<Tail>> = sizes
        .iter()
        .map(|_| Vec::with_capacity(P99_GROUPS))
        .collect();
    for group in 0..P99_GROUPS {
        for step in 0..sizes.len() {
            let at = (step + group) % sizes.len();
            if let (Some(&n), Some(row)) = (sizes.get(at), out.get_mut(at)) {
                row.push(measure(at, n, group));
            }
        }
    }
    out
}

/// The median of the per-group ratios `at[g] / base[g]`, in thousandths, with
/// the smallest and largest beside it.
///
/// `None` when the two sides hold different numbers of groups, when there are
/// none, or when any sample timed at zero: a ratio over an operation the
/// optimiser deleted is not a measurement, as [`ratio`] says.
fn median_ratio_permille(base: &[u128], at: &[u128]) -> Option<(u128, u128, u128)> {
    if base.len() != at.len() || base.is_empty() || base.contains(&0) || at.contains(&0) {
        return None;
    }
    let mut ratios: Vec<u128> = base.iter().zip(at).map(|(b, a)| a * 1_000 / b).collect();
    ratios.sort_unstable();
    Some((
        ratios.get(ratios.len() / 2).copied()?,
        ratios.first().copied()?,
        ratios.last().copied()?,
    ))
}

/// A fixed-seed generator for the probed positions. `SplitMix64`'s finaliser.
fn mix(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Is a median ratio inside [`CEILING_PERMILLE`] in either direction?
///
/// Two-sided for the reason [`ratio`] gives: an operation that got CHEAPER
/// with the input is as much a dependence on it as one that got dearer.
fn within_ceiling(median_permille: u128) -> bool {
    median_permille > 0
        && median_permille <= CEILING_PERMILLE
        && 1_000_000 / median_permille <= CEILING_PERMILLE
}

/// The median of one field over a size's groups.
fn median_of(tails: &[Tail], field: impl Fn(&Tail) -> u128) -> u128 {
    let mut v: Vec<u128> = tails.iter().map(field).collect();
    v.sort_unstable();
    v.get(v.len() / 2).copied().unwrap_or(0)
}

/// Prints one interleaved row and returns whether it held.
///
/// Per size: the median over groups of p50 and of p99, the largest max of any
/// group, and -- past the base -- the median paired p99 ratio with the range of
/// the nine. A size `gated` refuses returns `true` after printing.
fn interleaved_row(
    label: &str,
    unit: &str,
    sizes: &[usize],
    tails: &[Vec<Tail>],
    gated: impl Fn(usize) -> bool,
) -> bool {
    let Some(base) = tails.first() else {
        refuse(&format!("{label}: no size was measured"))
    };
    let base_p99: Vec<u128> = base.iter().map(|t| t.p99).collect();
    let mut ok = true;
    for (&n, groups) in sizes.iter().zip(tails) {
        println!(
            "  {label:<36} n={n:>9}  p50 {:>8} ns  p99 {:>8} ns  max {:>9} ns  per {unit}",
            median_of(groups, |t| t.p50),
            median_of(groups, |t| t.p99),
            groups.iter().map(|t| t.max).max().unwrap_or(0),
        );
        if n == sizes.first().copied().unwrap_or(0) {
            continue;
        }
        let at_p99: Vec<u128> = groups.iter().map(|t| t.p99).collect();
        let Some((median, low, high)) = median_ratio_permille(&base_p99, &at_p99) else {
            println!("  {label} p99 UNMEASURABLE — a group timed at zero");
            ok = false;
            continue;
        };
        let gate = gated(n);
        let held = !gate || within_ceiling(median);
        println!(
            "  {label} p99, {n} against {}: median of {P99_GROUPS} paired ratios \
             {}.{:03}x (range {}.{:03}x..{}.{:03}x)  {}",
            sizes.first().copied().unwrap_or(0),
            median / 1_000,
            median % 1_000,
            low / 1_000,
            low % 1_000,
            high / 1_000,
            high % 1_000,
            match (gate, held) {
                (false, _) => "REPORTED, not gated",
                (true, true) => "ok",
                (true, false) => "BREACH",
            }
        );
        ok &= held;
    }
    ok
}

/// The statistic the p99 rows gate is itself tested, on fixed numbers, before
/// any row trusts it (D-4482).
///
/// A gate that cannot fail is not a gate, and one that fails on noise is the
/// so1-3 finding. Nine quiet base groups against: four groups disturbed
/// tenfold in either direction (noise: passes); every group four times
/// dearer (an O(n) operation: fails); five of nine disturbed (a majority:
/// fails); every group four times cheaper (fails, two-sided); and a zero
/// sample (unmeasurable: refused).
fn the_interleaved_statistic_can_pass_and_can_fail() -> bool {
    let quiet = [100_u128; P99_GROUPS];
    let noisy = [1_000, 100, 100, 1_000, 100, 10, 100, 100, 1_000];
    let grown = [400_u128; P99_GROUPS];
    let shrunk = [25_u128; P99_GROUPS];
    let majority = [1_000, 1_000, 1_000, 1_000, 1_000, 100, 100, 100, 100];
    let mut zero = quiet;
    if let Some(first) = zero.first_mut() {
        *first = 0;
    }
    let verdict =
        |at: &[u128]| median_ratio_permille(&quiet, at).map(|(m, _, _)| within_ceiling(m));
    let checks = [
        (
            "four of nine groups disturbed tenfold passes",
            verdict(&noisy),
            Some(true),
        ),
        (
            "a quiet base against itself passes",
            verdict(&quiet),
            Some(true),
        ),
        (
            "every group four times dearer fails",
            verdict(&grown),
            Some(false),
        ),
        (
            "every group four times cheaper fails",
            verdict(&shrunk),
            Some(false),
        ),
        (
            "five of nine disturbed fails",
            verdict(&majority),
            Some(false),
        ),
        ("a zero sample is unmeasurable", verdict(&zero), None),
        (
            "unequal group counts are unmeasurable",
            median_ratio_permille(&quiet, &quiet[1..]).map(|(m, _, _)| within_ceiling(m)),
            None,
        ),
    ];
    let mut ok = true;
    for (what, got, want) in checks {
        let held = got == want;
        println!(
            "  p99 statistic self-test: {what:<48} {}",
            if held { "ok" } else { "BROKEN" }
        );
        ok &= held;
    }
    ok
}

/// O1P-03 and O1P-04 — k=1 duplicate rejection and result append are flat AT
/// p99 from 10^3 to 10^6, not only at the minimum of a mean (D-3300).
///
/// C-E-10 rejects position ZERO twenty thousand times, so it times one bucket
/// the cache already holds; a table whose probe sequences lengthened with its
/// load would stay green there. O1P-03 rejects a different, uniformly drawn,
/// already-offered position on every operation, through the production
/// [`engine::primitives::offer`]. O1P-04 pushes through
/// [`engine::primitives::append`] into a vector reserved before the pushes —
/// the reservation production makes before a level — so the distribution is
/// of pushes that fit, which is the claim C-E-11 states.
///
/// Since D-4482 (so1-3) the gated O1P-04 row holds exactly `n` results at
/// every timed push, its pages written by the warm-up, so what it gates is
/// the push and nothing else. The first push into an untouched page takes a
/// minor fault (D-3301: 3,019 ns against 780 ns per 32 pushes with the pages
/// touched); that shape is printed beside it as `first touch` and not gated,
/// because whether a reservation's pages are fresh is the allocator's state,
/// not a property of `n`. `docs/06-limits.md` states both.
///
/// Both rows are measured in [`interleaved`] groups and gated on the median
/// paired ratio since D-4482; see [`P99_GROUPS`].
fn rule_four_primitives_are_flat_at_p99() -> bool {
    let one = Itemset {
        mask: candidate(3),
        hits: 1,
    };
    let mut sets: Vec<std::collections::HashSet<u32>> =
        P99_SIZES.iter().map(|&n| offered_of(n)).collect();
    let dup = interleaved(&P99_SIZES, |at, n, group| {
        let Some(set) = sets.get_mut(at) else {
            refuse("O1P-03: no offered set for a measured size")
        };
        let width = u64::try_from(n).unwrap_or(1).max(1);
        group_tail(P99_GROUP_SAMPLES, P99_GROUP_SAMPLES, group as u64, |call| {
            let mut rejected = 0usize;
            for at in 0..P99_BATCH as u64 {
                let position = u32::try_from(mix(call.wrapping_mul(64) ^ at) % width).unwrap_or(0);
                if !engine::primitives::offer(set, black_box(position)) {
                    rejected = rejected.saturating_add(1);
                }
            }
            rejected
        })
    });
    drop(sets);
    // HELD EXACTLY `n` AT EVERY TIMED PUSH (D-4482). Each sample truncates
    // back to `n` -- O(1) for a `Copy` element -- before its pushes, so the
    // size the row names is the size every push meets. The rows used to let
    // the vector grow by every push of every round, so the 10^3 base ended its
    // rounds holding 1.6 million and an O(n) append could not separate the
    // sizes; a scan planted on every 1,024th push read 1.117x at 10^4 and
    // 2.062x at 10^5 that way, breaching only at 10^6. The
    // `n` held are built with `vec!`, not through `append`, so a planted O(n)
    // append does not make the setup quadratic. The warm-up samples write the
    // reservation's pages before any timed push.
    let push = interleaved(&P99_SIZES, |_, n, group| {
        let mut out: Vec<Itemset> = vec![one; n];
        out.reserve_exact(P99_BATCH);
        group_tail(
            P99_GROUP_SAMPLES / 5,
            P99_GROUP_SAMPLES,
            group as u64,
            |_| {
                out.truncate(n);
                for _ in 0..P99_BATCH {
                    engine::primitives::append(&mut out, black_box(one));
                }
                out.len()
            },
        )
    });
    // Every push into a fresh page, the shape a level's first pushes have.
    let untouched = interleaved(&P99_SIZES, |_, n, group| {
        let mut out: Vec<Itemset> = vec![one; n];
        out.reserve_exact(P99_GROUP_SAMPLES * P99_BATCH);
        group_tail(0, P99_GROUP_SAMPLES, group as u64, |_| {
            for _ in 0..P99_BATCH {
                engine::primitives::append(&mut out, black_box(one));
            }
            out.len()
        })
    });
    // k=1 offers one position per live condition, so its table never
    // holds more than `ConditionMask::BITS` (384). 10^4 is 26x past that and
    // is gated. 10^5 was gated too until D-4482: there the table is 640 KiB,
    // and on a shared four-vCPU box it competes for the level-two cache with
    // whatever else runs, so its p99 moved between 2.3x and 5.3x of 10^3 with
    // the machine's load while the code stood still. It is printed, as 10^6
    // always was; `docs/06-limits.md` names both (D-3301, D-4482).
    let mut ok = interleaved_row(
        "O1P-03 k=1 duplicate rejection",
        "32 probes",
        &P99_SIZES,
        &dup,
        |n| n <= 10_000,
    );
    ok &= interleaved_row(
        "O1P-04 result append",
        "32 pushes",
        &P99_SIZES,
        &push,
        |_| true,
    );
    // THE FIRST TOUCH OF EACH PAGE, printed and not gated (D-4482). A push
    // into a page of the reservation nothing has written yet takes a minor
    // fault. Whether a reservation's pages are fresh depends on the
    // allocator, not on `n`: glibc hands a freed block below its mapping
    // threshold back with its pages already touched, and maps a block past
    // that threshold afresh. With every size's reservation sized alike, only
    // the 10^6 one crosses it, so only 10^6 faulted, and its p99 read about
    // 4x the others in every group -- a fact about the allocator's state that
    // a gate on the push cannot honestly carry.
    ok &= interleaved_row(
        "O1P-04 append, first touch",
        "32 pushes",
        &P99_SIZES,
        &untouched,
        |_| false,
    );
    ok
}

/// FXD-02 — one block of the production support pass costs the same per bar at
/// p99 whatever column it was cut from, 10^3 to 10^6 bars (so1-1, D-4481).
///
/// Each sample is one [`support_block`] call -- the unit
/// [`Column::support_each`] repeats -- over a block of
/// [`SUPPORT_BLOCK_ROWS`] rows drawn uniformly from the column, counting
/// [`FXD02_CANDIDATES`] candidates. The block is drawn at random, not walked
/// in order, so at 10^6 bars nearly every sample starts with its block out of
/// the cache: the row prices the fetch the walk pays once per block, which is
/// the cost that grew with the column when the walk counted one candidate per
/// pass. Every sample does the same work at every size, so the p99s are
/// directly comparable, and the row is gated two-sided at every size on the
/// median paired ratio.
fn one_support_block_is_flat_at_p99() -> bool {
    // The rows of each column. `Column` hands out no rows, and an accessor
    // added to production for a bench would be the wrong way round, so the
    // bench's own deterministic generator is asked again: `support_block` takes
    // a slice of rows, which is exactly what `support_each` hands it.
    let rows: Vec<Vec<ConditionMask>> = P99_SIZES.iter().map(|&n| column(n)).collect();
    let mut items: Vec<Itemset> = batch_candidates()
        .into_iter()
        .take(FXD02_CANDIDATES)
        .collect();
    let tails = interleaved(&P99_SIZES, |at, _, group| {
        let Some(rows) = rows.get(at) else {
            refuse("FXD-02: no column for a measured size")
        };
        let blocks = u64::try_from(rows.len() / SUPPORT_BLOCK_ROWS)
            .unwrap_or(1)
            .max(1);
        group_tail(FXD02_SAMPLES, FXD02_SAMPLES, group as u64, |call| {
            let start = usize::try_from(mix(call) % blocks).unwrap_or(0) * SUPPORT_BLOCK_ROWS;
            let block = rows.get(start..start + SUPPORT_BLOCK_ROWS).unwrap_or(&[]);
            support_block(black_box(block), &mut items);
            block.len()
        })
    });
    interleaved_row(
        "FXD-02 support block",
        "512 bars x 16 candidates",
        &P99_SIZES,
        &tails,
        |_| true,
    )
}

/// Candidates per FXD-02 sample. Sixteen keeps one sample near ten
/// microseconds: long against the timer, and short against a scheduler
/// slice, so a preemption lands in well under one sample in a hundred and a
/// shared box does not decide the p99.
const FXD02_CANDIDATES: usize = 16;

/// Samples per size inside one FXD-02 group.
const FXD02_SAMPLES: usize = 1_000;

/// FXD-03 — REPORTED, NOT GATED: one candidate per pass over the whole column,
/// the shape the walk counted in until D-4481 and the shape a lane still has
/// when a level's tail batch leaves it one candidate.
///
/// Its per-bar cost is the memory system's -- a whole column streamed for six
/// word-ANDs a bar -- and it grows as the column leaves each cache level; the
/// figure is printed so `docs/06-limits.md` can quote it rather than claim it.
fn one_candidate_per_pass_is_reported() -> bool {
    let columns: Vec<Column> = [10_000_usize, 100_000, 1_000_000]
        .iter()
        .map(|&n| owned_column(n))
        .collect();
    let sizes: Vec<usize> = columns
        .iter()
        .map(|c| usize::try_from(c.bars()).unwrap_or(0))
        .collect();
    let mask = candidate(3);
    let tails = interleaved(&sizes, |at, n, group| {
        let Some(owned) = columns.get(at) else {
            refuse("FXD-03: no column for a measured size")
        };
        let per_bar_group = group_tail(1, FXD03_SAMPLES, group as u64, |_| {
            let mut one = [Itemset { mask, hits: 0 }];
            owned.support_each(black_box(&mut one));
            usize::try_from(one.first().map_or(0, |item| item.hits)).unwrap_or(0)
        });
        let bars = u128::try_from(n).unwrap_or(1).max(1);
        // Picoseconds per bar, so sizes 100x apart print on one scale.
        Tail {
            p50: per_bar_group.p50 * 1_000 / bars,
            p99: per_bar_group.p99 * 1_000 / bars,
            max: per_bar_group.max * 1_000 / bars,
        }
    });
    interleaved_row(
        "FXD-03 one candidate per pass",
        "bar, x1000 (ps)",
        &sizes,
        &tails,
        |_| false,
    )
}

/// Samples per size inside one FXD-03 group.
const FXD03_SAMPLES: usize = 20;

/// FXD-08 — each level's two sorts, timed at the widths a sweep reaches
/// (W3-engine1-1, D-4483).
///
/// A level is ordered once by [`engine::primitives::sort_level`], the
/// canonical sort the walk publishes it in, and indexed once by
/// [`engine::primitives::JoinProbe::try_new`], whose keyed prefix vector is
/// the second sort (plus the membership set). Both are comparison sorts, so
/// a level costs O(|F| log |F|) beyond its join: per level, never per bar
/// and never per pair. `docs/06-limits.md` said "not timed here"; this row is
/// the timing it quotes.
///
/// The fixture is |F| distinct three-bit masks spread over 370 positions,
/// handed to the sort in a scrambled order (the order a join emits is not
/// the canonical one) and to the index already sorted (the order the walk
/// hands it the previous level in). Every size runs inside every one of
/// [`FXD08_GROUPS`] groups, rotated, as the p99 rows do (D-4482).
///
/// GATED: the median over groups of the paired ratio of the group's median
/// time per item per log2 |F|, each wider frontier against 10^3, must stay
/// at or under [`FXD08_CEILING_PERMILLE`]. It is NOT flat, and the row does
/// not pretend it is: on the box D-4483 measured, the sort's figure rose
/// about 2x, 4x and 6x at 10^4, 10^5 and 10^6, because a wider level of
/// these masks ties on more leading words (each comparison reads further
/// into its seven-word key) and moves 56-byte items through caches it no
/// longer fits. Both effects are bounded -- a comparison reads at most seven
/// words -- and the ceiling is that bound on top of the usual three: a sort
/// that went quadratic multiplies the figure by |F| / log |F|, at least 60x
/// at 10^5, and fails. One-sided, because fixed per-call work amortised over
/// a wider level makes the figure FALL, which is not a dependence on |F|.
fn each_level_sort_is_n_log_n() -> bool {
    let scrambled: Vec<Vec<Itemset>> = FXD08_SIZES.iter().map(|&w| frontier_of(w)).collect();
    let canonical: Vec<Vec<Itemset>> = scrambled
        .iter()
        .map(|level| {
            let mut sorted = level.clone();
            engine::primitives::sort_level(&mut sorted);
            sorted
        })
        .collect();
    // The fixture is what the row says it is: distinct masks, a sort that
    // orders them, and an index that keeps every one.
    for (level, sorted) in scrambled.iter().zip(&canonical) {
        let ordered = sorted.is_sorted_by_key(|i| (i.mask.words(), i.hits));
        let mut masks: Vec<[u64; 6]> = level.iter().map(|i| i.mask.words()).collect();
        masks.sort_unstable();
        masks.dedup();
        let width = engine::primitives::JoinProbe::try_new(sorted)
            .ok()
            .map(|probe| probe.width());
        if !ordered || masks.len() != level.len() || width != Some(level.len()) {
            refuse("FXD-08: the fixture is not |F| distinct masks the sort orders");
        }
    }
    let widest = FXD08_SIZES.iter().copied().max().unwrap_or(0);
    let mut work: Vec<Itemset> = Vec::with_capacity(widest);
    let sizes = FXD08_SIZES.len();
    let mut sort_ns: Vec<Vec<u128>> = vec![Vec::new(); sizes];
    let mut index_ns: Vec<Vec<u128>> = vec![Vec::new(); sizes];
    let mut sort_groups: Vec<Vec<u128>> = vec![Vec::with_capacity(FXD08_GROUPS); sizes];
    let mut index_groups: Vec<Vec<u128>> = vec![Vec::with_capacity(FXD08_GROUPS); sizes];
    for group in 0..FXD08_GROUPS {
        for step in 0..sizes {
            let at = (step + group) % sizes;
            let (Some(level), Some(sorted), Some(&samples)) =
                (scrambled.get(at), canonical.get(at), FXD08_SAMPLES.get(at))
            else {
                refuse("FXD-08: no fixture for a measured size")
            };
            let mut sorting = Vec::with_capacity(samples);
            let mut indexing = Vec::with_capacity(samples);
            for _ in 0..samples {
                work.clear();
                work.extend_from_slice(level);
                let start = Instant::now();
                engine::primitives::sort_level(black_box(&mut work));
                sorting.push(start.elapsed().as_nanos());
                black_box(work.first());

                let start = Instant::now();
                let probe = engine::primitives::JoinProbe::try_new(black_box(sorted));
                indexing.push(start.elapsed().as_nanos());
                // Dropped outside the timed span: freeing the index is not
                // building it.
                black_box(probe.map(|p| p.width()).ok());
            }
            let per_item_log = |ns: &mut Vec<u128>| -> u128 {
                ns.sort_unstable();
                rank(ns, 500) * 1_000 / item_log(level.len())
            };
            if let (Some(all_sort), Some(all_index), Some(sg), Some(ig)) = (
                sort_ns.get_mut(at),
                index_ns.get_mut(at),
                sort_groups.get_mut(at),
                index_groups.get_mut(at),
            ) {
                all_sort.extend_from_slice(&sorting);
                all_index.extend_from_slice(&indexing);
                sg.push(per_item_log(&mut sorting));
                ig.push(per_item_log(&mut indexing));
            }
        }
    }
    let sort_ok = level_sort_row("FXD-08 level sort", &mut sort_ns, &sort_groups);
    let index_ok = level_sort_row("FXD-08 level index", &mut index_ns, &index_groups);
    sort_ok && index_ok
}

/// Frontier widths FXD-08 sorts and indexes: 10^3 to 10^6 survivors.
///
/// 10^6 is a level of 56 MB of `Itemset`s, past which `Ladder`'s own memory
/// ceiling refuses before a sort is reached on most machines this runs on.
const FXD08_SIZES: [usize; 4] = [1_000, 10_000, 100_000, 1_000_000];

/// Samples per size inside one FXD-08 group, fewer as one sort grows: a
/// 10^6-wide sort takes tenths of a second on the box D-4483 measured.
const FXD08_SAMPLES: [usize; 4] = [200, 40, 8, 2];

/// FXD-08 groups.
const FXD08_GROUPS: usize = 5;

/// The FXD-08 ceiling: seven times [`CEILING_PERMILLE`], one for each word a
/// level-sort comparison may read (six mask words and `hits`). The index's
/// keyed pairs compare up to twelve words and are held to the same seven.
const FXD08_CEILING_PERMILLE: u128 = 7 * CEILING_PERMILLE;

/// `width x floor(log2 width)`, the comparison count a level sort scales by.
fn item_log(width: usize) -> u128 {
    let width = u128::try_from(width).unwrap_or(1).max(2);
    width * u128::from(width.ilog2())
}

/// |F| distinct three-bit masks over positions 0..370, in a scrambled order.
///
/// Position `i % 128`, then `128 + (i / 128) % 128`, then
/// `256 + (i / 16_384) % 114`: injective below 1,867,776, so every mask is
/// distinct at every width this row uses.
fn frontier_of(width: usize) -> Vec<Itemset> {
    let mut keyed: Vec<(u64, Itemset)> = (0..width)
        .map(|i| {
            let n = u32::try_from(i).unwrap_or(u32::MAX);
            let mask = ConditionMask::default()
                .with_bit(n % 128)
                .with_bit(128 + (n / 128) % 128)
                .with_bit(256 + (n / 16_384) % 114);
            let seed = i as u64;
            (
                mix(seed),
                Itemset {
                    mask,
                    hits: mix(!seed) % 100_000,
                },
            )
        })
        .collect();
    keyed.sort_unstable_by_key(|(key, _)| *key);
    keyed.into_iter().map(|(_, item)| item).collect()
}

/// Prints one FXD-08 row: per width p50 / p99 / max of every sample, then
/// the median paired ratio of the per-item-per-log2 figure against 10^3.
fn level_sort_row(label: &str, all: &mut [Vec<u128>], groups: &[Vec<u128>]) -> bool {
    let Some(base) = groups.first() else {
        refuse(&format!("{label}: no size was measured"))
    };
    let mut ok = true;
    for ((&width, ns), per) in FXD08_SIZES.iter().zip(all.iter_mut()).zip(groups) {
        ns.sort_unstable();
        println!(
            "  {label:<22} |F|={width:>9}  p50 {:>11} ns  p99 {:>11} ns  max {:>11} ns  \
             ({} samples)  p50 {} ps per item x log2|F|",
            rank(ns, 500),
            rank(ns, 990),
            ns.last().copied().unwrap_or(0),
            ns.len(),
            median_of_values(per),
        );
        if width == FXD08_SIZES.first().copied().unwrap_or(0) {
            continue;
        }
        let Some((median, low, high)) = median_ratio_permille(base, per) else {
            println!("  {label} UNMEASURABLE — a group timed at zero");
            ok = false;
            continue;
        };
        let held = median <= FXD08_CEILING_PERMILLE;
        println!(
            "  {label} per item x log2|F|, {width} against {}: median of {FXD08_GROUPS} \
             paired ratios {}.{:03}x (range {}.{:03}x..{}.{:03}x), ceiling {}x  {}",
            FXD08_SIZES.first().copied().unwrap_or(0),
            median / 1_000,
            median % 1_000,
            low / 1_000,
            low % 1_000,
            high / 1_000,
            high % 1_000,
            FXD08_CEILING_PERMILLE / 1_000,
            if held { "ok" } else { "BREACH" }
        );
        ok &= held;
    }
    ok
}

/// The median of a list of per-group figures.
fn median_of_values(values: &[u128]) -> u128 {
    let mut v = values.to_vec();
    v.sort_unstable();
    v.get(v.len() / 2).copied().unwrap_or(0)
}

/// C-E-08 — one pair of the join costs the same whatever the frontier holds.
///
/// # The row `DEFAULT_PAIR_BUDGET` cited before it existed
///
/// `engine::DEFAULT_PAIR_BUDGET`'s doc said the per-pair cost was "measured by
/// `C-E-08`". **There was no C-E-08.** The rows ran 01–07 and 09, so the budget —
/// the thing that bounds a sweep's wall time — was stated in a unit nothing
/// measured. An audit found it; that is exactly the defect CI gate 12 exists to
/// catch, and it was written while fixing gate-12 defects.
///
/// # And then it measured a stand-in, which is the same defect again
///
/// The first version of this row timed `a.union(b).popcount()` over a
/// bench-local vector, under a comment saying that was "exactly as
/// `next_level` does". It was not: production performs no popcount per pair,
/// and what it does perform -- the subset prune and the meaning prune -- was
/// never timed. Since D-0924 the row walks the production per-pair screen
/// through [`engine::primitives::JoinProbe`], over frontiers indexed by the
/// production `JoinIndex`.
///
/// The budget is in pair iterations rather than seconds because seconds are a
/// claim about a machine. That only works if a pair costs the same everywhere in
/// the walk at a given depth, which is what this measures: every 2-subset of 15
/// positions (105 masks, 455 pairs) against every 2-subset of 45 (990 masks,
/// 14,190 pairs), all k=3 candidates with every subset frequent, so each pair
/// performs the union, its one non-parent subset probe and the meaning prune.
/// The per-pair cost DOES grow with depth, through the subset prune; that is
/// C-E-12's row and `docs/06-limits.md`'s, not a defect here.
fn one_join_pair_costs_the_same_at_every_frontier_width() -> bool {
    let pair_ps = |positions: u32| -> Option<u128> {
        let frontier: Vec<Itemset> = (0..positions)
            .flat_map(|low| {
                (low.saturating_add(1)..positions).map(move |high| Itemset {
                    mask: ConditionMask::default().with_bit(low).with_bit(high),
                    hits: 1,
                })
            })
            .collect();
        let probe = engine::primitives::JoinProbe::try_new(&frontier).ok()?;
        let (pairs, survivors) = probe.screen_every_pair();
        // C(m, 3) pairs, each one a k=3 candidate whose every subset is in the
        // frontier. Anything else means the fixture is not measuring a pair.
        let m = u64::from(positions);
        let expected = m * m.saturating_sub(1) * m.saturating_sub(2) / 6;
        if pairs != expected || survivors == 0 || probe.width() != frontier.len() {
            return None;
        }
        let reps = (2_000_000 / pairs).max(1);
        let total = once_ps(|| {
            let mut acc = 0_u64;
            for _ in 0..reps {
                acc = acc.wrapping_add(black_box(&probe).screen_every_pair().1);
            }
            black_box(acc)
        });
        Some(total / u128::from(pairs.saturating_mul(reps)).max(1))
    };

    match (pair_ps(15), pair_ps(45)) {
        (Some(narrow), Some(wide)) => ratio(
            "C-E-08 one join pair: 105-wide frontier -> 990-wide",
            narrow,
            wide,
        ),
        _ => refuse("C-E-08: the join probe did not walk the pairs the fixture builds"),
    }
}

/// C-E-12 — one subset-prune probe costs the same at every depth.
///
/// # Why this row exists, and why it is a per-PROBE row
///
/// The subset prune makes one `MaskSet` probe per set bit of a candidate
/// below its two parents, so a candidate that survives costs `k - 2` probes:
/// the per-candidate cost is Theta(k), bounded by the 384-bit mask but not
/// constant. It had no row, and its own doc called it both "O(1)" and "a
/// 384-probe loop". `docs/06-limits.md` now names the Theta(k); this row pins
/// the part that must not drift, the cost of ONE probe, from k=4 (2 probes) to
/// k=320 (318 probes). The frontier is every (k-1)-subset of one k-set, so the
/// join forms exactly one pair and every probe hits. The ratio is one-sided
/// because the union and the meaning prune are fixed work amortised over more
/// probes at larger k. D-0924.
fn one_subset_probe_costs_the_same_at_every_depth() -> bool {
    let probe_ps = |k: u32| -> Option<u128> {
        let set = (0..k).fold(ConditionMask::default(), ConditionMask::with_bit);
        let frontier: Vec<Itemset> = (0..k)
            .map(|dropped| Itemset {
                mask: set.without_bit(dropped),
                hits: 1,
            })
            .collect();
        let probe = engine::primitives::JoinProbe::try_new(&frontier).ok()?;
        if probe.screen_every_pair() != (1, 1) {
            return None;
        }
        let probes = u64::from(k.saturating_sub(2)).max(1);
        let reps = (2_000_000 / probes).max(1);
        let total = once_ps(|| {
            let mut acc = 0_u64;
            for _ in 0..reps {
                acc = acc.wrapping_add(black_box(&probe).screen_every_pair().1);
            }
            black_box(acc)
        });
        Some(total / u128::from(probes.saturating_mul(reps)).max(1))
    };
    match (probe_ps(4), probe_ps(320)) {
        (Some(shallow), Some(deep)) => {
            growth_ratio("C-E-12 one subset probe: k=4 -> k=320", shallow, deep)
        }
        _ => refuse("C-E-12: the join probe did not form the single all-frequent pair"),
    }
}

/// C-E-09 — the live support path is flat across the mask's entire width.
///
/// C-E-02 follows the deepest ordinary fixture from k=1 to k=8. A defect keyed
/// above that range could pass it while production is free to reach deeper levels.
/// This row drives the other endpoint: every one of the mask's 384 positions is
/// required. `Column::support` must still perform one six-word `hits` operation per
/// bar, never one operation per required position.
fn live_support_is_flat_across_the_entire_mask_width() -> bool {
    let column = owned_column(100_000);
    let one = candidate(1);
    let every = ConditionMask::from_words([u64::MAX; 6]);
    ratio(
        "C-E-09 Column::support: k=1 -> k=384",
        support_ps_per_bar(&column, &one),
        support_ps_per_bar(&column, &every),
    )
}

/// C-E-03 — the per-bar cost does not depend on the answer.
///
/// A column where every bar matches against one where none does. `support` folds
/// a branchless `hits` with no early exit, so both must cost the same. A version
/// that short-circuited would be fast on selective candidates and the sweep's
/// runtime would track the market rather than the bar count — a bound measured on
/// one kind of session would be wrong on another.
fn support_costs_the_same_whether_bars_match_or_not() -> bool {
    let n = 100_000;
    let mask = candidate(DRAWN_FROM.len());
    let all = Column::from_rows(&column_all_set(n));
    let none = Column::from_rows(&vec![ConditionMask::ZERO; n]);

    // Confirm the two columns really are the two extremes, or the row measures
    // nothing: a "no match" column that happened to match would make the ratio
    // meaningless while still printing a number.
    let hits_all = all.support(&mask);
    let hits_none = none.support(&mask);
    let want = u64::try_from(n).unwrap_or(u64::MAX);
    if hits_all != want || hits_none != 0 {
        refuse(&format!(
            "the columns are not the extremes they must be: {hits_all} of {n} and \
             {hits_none} of {n}"
        ));
    }
    println!(
        "  {:<60} {hits_all} of {n} -> {hits_none} of {n}",
        "C-E-03 matches counted (context)"
    );

    ratio(
        "C-E-03 support: every bar matches -> no bar matches",
        support_ps_per_bar(&all, &mask),
        support_ps_per_bar(&none, &mask),
    )
}

/// C-E-07 — the fingerprinted support path does not depend on the answer.
///
/// # Why C-E-03 was not enough, and was measuring nothing
///
/// C-E-03 now measures the exact `Column::support` method the sweep calls. This
/// separate row retains coverage for `support_fingerprinted`, whose 64-bar packing
/// and two folds must likewise process every bar and every packed word.
fn fingerprinted_support_costs_the_same_whether_bars_match_or_not() -> bool {
    let n = 100_000;
    let mask = candidate(DRAWN_FROM.len());
    let all = Column::from_rows(&column_all_set(n));
    let none = Column::from_rows(&vec![ConditionMask::ZERO; n]);

    // Both extremes, confirmed, or the row prints a number that means nothing.
    let hits_all = all.support(&mask);
    let hits_none = none.support(&mask);
    let want = u64::try_from(n).unwrap_or(u64::MAX);
    if hits_all != want || hits_none != 0 {
        refuse(&format!(
            "the owned columns are not the extremes they must be: \
             {hits_all} of {n} and {hits_none} of {n}"
        ));
    }
    let per_bar = |c: &Column| -> u128 {
        let bars = u128::try_from(n).unwrap_or(1).max(1);
        once_ps(|| black_box(c.support_fingerprinted(black_box(&mask)))) / bars
    };
    ratio(
        "C-E-07 Column::support_fingerprinted: every bar -> none",
        per_bar(&all),
        per_bar(&none),
    )
}

/// C-E-04 — a whole ladder walk's per-bar cost does not grow with column length.
///
/// The end-to-end row. `Ladder::walk` generates candidates, prunes subsets,
/// rejects duplicates and counts support, and the per-bar factor across all of
/// that must not drift, for the same reason C-E-01's must not: the sweep runs on 1.22
/// million bars, and a constant that drifts turns a linear pass into a
/// superlinear one.
///
/// The O(|frontier|²) level join is NOT claimed to be constant. With the live
/// set and frontier shape fixed it is a fixed cost in this comparison, so it is
/// expected to amortise and make the larger column cheaper per bar. The gate is
/// consequently one-sided: growth fails; improvement does not.
fn a_ladder_walk_does_not_get_dearer_per_bar() -> bool {
    let live: Vec<u32> = DRAWN_FROM.to_vec();

    let walk_ps_per_bar = |n: usize| -> u128 {
        let bars = column(n);
        let count = u128::try_from(n).unwrap_or(1).max(1);
        once_ps(|| Ladder::with_min_hits(1).walk(black_box(&bars), black_box(&live))) / count
    };

    // Prove BOTH timed shapes do the same non-vacuous structural work before
    // permitting the larger input to be cheaper. Hits may scale with the bar
    // count; the masks, depth, completion and per-level accounting may not.
    let small_probe = Ladder::with_min_hits(1).walk(&column(10_000), &live);
    let large_probe = Ladder::with_min_hits(1).walk(&column(100_000), &live);
    if small_probe.depth() == 0 || large_probe.depth() == 0 {
        refuse("one ladder reached no depth, so there is nothing comparable to measure");
    }
    if !small_probe.completed() || !large_probe.completed() {
        refuse("one ladder halted, so the two timed walks do not prove an extinction cost");
    }
    if small_probe.levels.len() != large_probe.levels.len()
        || small_probe
            .levels
            .iter()
            .zip(&large_probe.levels)
            .any(|(small, large)| {
                small.k != large.k
                    || small.generated != large.generated
                    || small.duplicates != large.duplicates
                    || small.excluded != large.excluded
                    || small.pruned != large.pruned
                    || small.infrequent != large.infrequent
                    || small.frequent.len() != large.frequent.len()
                    || small
                        .frequent
                        .iter()
                        .zip(&large.frequent)
                        .any(|(a, b)| a.mask != b.mask)
            })
    {
        refuse("the 10,000- and 100,000-bar walks produced different frontier topology");
    }
    println!(
        "  {:<60} depth {}, {} frequent set(s)",
        "C-E-04 what the walk actually found (context)",
        small_probe.depth(),
        small_probe.all_frequent().count()
    );
    for level in small_probe.levels.iter().chain(&large_probe.levels) {
        if !level.reconciles() {
            refuse(&format!(
                "the frontier at k={} does not reconcile, so the walk is not sound \
                 and its cost is not worth measuring",
                level.k
            ));
        }
    }

    // 1,000,000 is deliberately omitted here: a full walk over a million bars at
    // eight live positions takes minutes, and gate 8 runs on every push. The
    // 10,000 -> 100,000 step is a 10x change, which is enough to see a drifting
    // constant. Stated rather than implied, per §3 rule 6.
    growth_ratio(
        "C-E-04 walk: 10,000 bars -> 100,000 bars",
        walk_ps_per_bar(10_000),
        walk_ps_per_bar(100_000),
    )
}

/// C-E-06 — the owned live path costs the same class as its fixed-width reference.
///
/// Both functions perform one six-word hit test per bar and return the same count;
/// the owned wrapper exists for reuse across threshold probes, not to add another
/// candidate-dependent pass. This comparison catches uniform wrapper overhead that
/// the k-ratios would divide away, while C-E-05 remains the absolute floor budget.
fn live_column_costs_like_the_fixed_width_reference() -> bool {
    let bars = column(100_000);
    let column = Column::from_rows(&bars);
    let n = u128::try_from(bars.len()).unwrap_or(1).max(1);
    let mut ok = true;
    for k in [1_usize, DRAWN_FROM.len()] {
        let candidate = candidate(k);
        let reference_hits = support(&bars, &candidate);
        let live_hits = column.support(&candidate);
        if reference_hits != live_hits {
            refuse(&format!(
                "C-E-06 support counts disagree at k={k}: reference {reference_hits}, \
                 live {live_hits}"
            ));
        }
        let reference = once_ps(|| support(black_box(&bars), black_box(&candidate))) / n;
        let live = support_ps_per_bar(&column, &candidate);
        ok &= ratio(
            &format!("C-E-06 fixed-width reference -> live column at k={k}"),
            reference,
            live,
        );
    }
    ok
}

fn main() {
    println!("engine — gate 8, ceiling {CEILING_PERMILLE} permille");
    println!(
        "  ladder is {} byte(s) — no room for a depth field; mask is {} bits",
        core::mem::size_of::<Ladder>(),
        ConditionMask::BITS
    );
    let mut ok = the_interleaved_statistic_can_pass_and_can_fail();
    ok &= support_stays_within_its_budget();
    ok &= live_column_costs_like_the_fixed_width_reference();
    ok &= support_costs_the_same_per_bar_at_every_column_length();
    ok &= support_costs_the_same_per_bar_at_every_depth();
    ok &= support_costs_the_same_whether_bars_match_or_not();
    ok &= fingerprinted_support_costs_the_same_whether_bars_match_or_not();
    ok &= one_join_pair_costs_the_same_at_every_frontier_width();
    ok &= one_subset_probe_costs_the_same_at_every_depth();
    ok &= duplicate_rejection_costs_the_same_however_much_is_seen();
    ok &= result_append_costs_the_same_however_many_are_held();
    ok &= rule_four_primitives_are_flat_at_p99();
    ok &= one_support_block_is_flat_at_p99();
    ok &= one_candidate_per_pass_is_reported();
    ok &= each_level_sort_is_n_log_n();
    ok &= live_support_is_flat_across_the_entire_mask_width();
    ok &= a_ladder_walk_does_not_get_dearer_per_bar();
    if ok {
        println!("all ratios within the ceiling");
    } else {
        println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
        std::process::exit(1);
    }
}
