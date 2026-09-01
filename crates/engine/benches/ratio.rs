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

use engine::column::Column;
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

/// Cost per bar, in picoseconds, of the support method the live ladder calls.
fn support_ps_per_bar(column: &Column, mask: &ConditionMask) -> u128 {
    let n = u128::from(column.bars()).max(1);
    once_ps(|| column.support(black_box(mask))) / n
}

/// C-E-01 — the per-bar cost of support counting does not grow with the column.
///
/// Support counting is O(bars) by definition. This row is about the CONSTANT in
/// front of it: 1,000,000 bars must cost 100 times what 10,000 do, not more. A
/// drifting constant is how an O(bars) measurement becomes superlinear without
/// anything in the source looking like a nested loop.
fn support_costs_the_same_per_bar_at_every_column_length() -> bool {
    let mask = candidate(3);
    let mut sizes = COLUMNS.iter();
    let Some(first) = sizes.next() else {
        refuse("COLUMNS is empty, so there is no baseline")
    };
    let base = support_ps_per_bar(&owned_column(*first), &mask);

    let mut ok = true;
    for n in sizes {
        ok &= ratio(
            &format!("C-E-01 support: {first} bars -> {n} bars"),
            base,
            support_ps_per_bar(&owned_column(*n), &mask),
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

/// `n` distinct offered positions, pre-sized as production k=1 pre-sizes them.
fn offered_of(n: usize) -> std::collections::HashSet<u32> {
    let mut set = std::collections::HashSet::with_capacity(n);
    let end = u32::try_from(n).unwrap_or(u32::MAX);
    for position in 0..end {
        set.insert(position);
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
                if !black_box(&mut *set).insert(black_box(0)) {
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
                out.push(black_box(one));
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
/// The budget is in pair iterations rather than seconds because seconds are a
/// claim about a machine. That only works if a pair costs the same everywhere in
/// the walk, which is what this measures: a mask union and a popcount, against a
/// frontier ten times larger. If the per-pair cost drifted with frontier size the
/// budget would mean different amounts of work at different depths, and a bound
/// that changes meaning is not a bound.
fn one_join_pair_costs_the_same_at_every_frontier_width() -> bool {
    let pair_ps = |width: usize| -> u128 {
        // A frontier of `width` distinct single-bit masks, joined pairwise
        // exactly as `next_level` does: union, then popcount.
        let frontier: Vec<ConditionMask> = (0..width)
            .map(|i| {
                let bit = u32::try_from(i).unwrap_or(0) % ConditionMask::BITS;
                ConditionMask::default().with_bit(bit)
            })
            .collect();
        let pairs = u128::try_from(width.saturating_mul(width.saturating_sub(1)) / 2)
            .unwrap_or(1)
            .max(1);
        let total = once_ps(|| {
            let mut acc = 0_u32;
            for (i, a) in frontier.iter().enumerate() {
                for b in frontier.iter().skip(i.saturating_add(1)) {
                    acc = acc.wrapping_add(black_box(a).union(black_box(b)).popcount());
                }
            }
            black_box(acc)
        });
        total / pairs
    };

    ratio(
        "C-E-08 one join pair: 100-wide frontier -> 1,000-wide",
        pair_ps(100),
        pair_ps(1_000),
    )
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
    let mut ok = true;
    ok &= support_stays_within_its_budget();
    ok &= live_column_costs_like_the_fixed_width_reference();
    ok &= support_costs_the_same_per_bar_at_every_column_length();
    ok &= support_costs_the_same_per_bar_at_every_depth();
    ok &= support_costs_the_same_whether_bars_match_or_not();
    ok &= fingerprinted_support_costs_the_same_whether_bars_match_or_not();
    ok &= one_join_pair_costs_the_same_at_every_frontier_width();
    ok &= duplicate_rejection_costs_the_same_however_much_is_seen();
    ok &= result_append_costs_the_same_however_many_are_held();
    ok &= live_support_is_flat_across_the_entire_mask_width();
    ok &= a_ladder_walk_does_not_get_dearer_per_bar();
    if ok {
        println!("all ratios within the ceiling");
    } else {
        println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
        std::process::exit(1);
    }
}
