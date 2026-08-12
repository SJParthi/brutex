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

use engine::{Ladder, support};
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
    println!("  the per-bar floor is {floor} ps — one black-boxed wrapping_add per bar");
    let mut ok = true;
    for k in [1usize, DRAWN_FROM.len()] {
        ok &= budget(
            &format!("C-E-05 support at k={k}"),
            floor,
            support_ps_per_bar(&bars, &candidate(k)),
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

/// Cost per bar, in picoseconds, of counting `mask`'s support over `bars`.
fn support_ps_per_bar(bars: &[ConditionMask], mask: &ConditionMask) -> u128 {
    let n = u128::try_from(bars.len()).unwrap_or(1).max(1);
    once_ps(|| support(black_box(bars), black_box(mask))) / n
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
    let base = support_ps_per_bar(&column(*first), &mask);

    let mut ok = true;
    for n in sizes {
        ok &= ratio(
            &format!("C-E-01 support: {first} bars -> {n} bars"),
            base,
            support_ps_per_bar(&column(*n), &mask),
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
    let bars = column(100_000);
    let base = support_ps_per_bar(&bars, &candidate(1));

    let mut ok = true;
    for k in [4, DRAWN_FROM.len()] {
        ok &= ratio(
            &format!("C-E-02 support: a k=1 candidate -> a k={k} candidate"),
            base,
            support_ps_per_bar(&bars, &candidate(k)),
        );
    }
    ok
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
    let all = column_all_set(n);
    let none = vec![ConditionMask::ZERO; n];

    // Confirm the two columns really are the two extremes, or the row measures
    // nothing: a "no match" column that happened to match would make the ratio
    // meaningless while still printing a number.
    let hits_all = support(&all, &mask);
    let hits_none = support(&none, &mask);
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

/// C-E-04 — a whole ladder walk costs the same per bar at every column length.
///
/// The end-to-end row. `Ladder::walk` generates candidates, prunes subsets,
/// rejects duplicates and counts support, and the per-bar factor across all of
/// that must not drift, for the same reason C-E-01's must not: the sweep runs on 1.22
/// million bars, and a constant that drifts turns a linear pass into a
/// superlinear one.
///
/// The O(|frontier|²) level join is NOT what this measures and is not claimed to
/// be constant — the live set is held fixed across the two columns so the
/// frontier shape is identical and only the bar count varies.
fn a_ladder_walk_costs_the_same_per_bar_at_every_column_length() -> bool {
    let live: Vec<u32> = DRAWN_FROM.to_vec();

    let walk_ps_per_bar = |n: usize| -> u128 {
        let bars = column(n);
        let count = u128::try_from(n).unwrap_or(1).max(1);
        once_ps(|| Ladder::with_min_hits(1).walk(black_box(&bars), black_box(&live))) / count
    };

    // Reported once so a reader can see the walk did real work rather than
    // returning an empty sweep, which would be fast and meaningless.
    let probe = Ladder::with_min_hits(1).walk(&column(10_000), &live);
    if probe.depth() == 0 {
        refuse("the ladder reached no depth, so there is nothing to measure");
    }
    println!(
        "  {:<60} depth {}, {} frequent set(s)",
        "C-E-04 what the walk actually found (context)",
        probe.depth(),
        probe.all_frequent().count()
    );
    for level in &probe.levels {
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
    ratio(
        "C-E-04 walk: 10,000 bars -> 100,000 bars",
        walk_ps_per_bar(10_000),
        walk_ps_per_bar(100_000),
    )
}

/// C-E-06 — the transposed column costs a fraction of the row-major walk.
///
/// Not a ratio between two inputs to one operation, like every row above, but a ratio
/// between two LAYOUTS answering the same question. That makes it the one row here that
/// can catch a broken transpose by cost rather than by answer: `column::tests` already
/// proves the two agree, and this proves the second one is worth having.
///
/// Measured on an arm64 laptop, release, `lto = "fat"`, 1,222,791 bars:
///
///     k=1  row-major 928 ps/bar  bitmaps  9.6 ps/bar   96.9x
///     k=2            906         bitmaps 18.1          50.0x
///     k=4            911         bitmaps 45.4          20.1x
///     k=8            891         bitmaps 74.5          12.0x
///
/// The row-major cost is flat in k -- that is what C-E-02 measures -- while the bitmap
/// cost grows with k, because k bitmaps must be ANDed. They would meet somewhere past
/// k=90, which no frontier reaches, so the transpose wins at every depth that runs.
///
/// The floor is FOUR, not twelve. The measured worst case in range is 12x at k=8 and the
/// budget leaves 3x for a different microarchitecture -- a gate that fails on a slower
/// runner is a gate that gets ignored. What it refuses is the case that matters: a
/// transpose that has quietly become a full-column walk again.
fn the_transposed_column_beats_the_row_major_walk() -> bool {
    /// The smallest speedup accepted at any k tested.
    const FLOOR: u128 = 4;

    let bars = column(100_000);
    let vertical = engine::column::Column::transpose(&bars);
    let mut ok = true;
    for k in [1_usize, 4, DRAWN_FROM.len()] {
        let cand = candidate(k);
        // Same answer, first. A faster wrong number is not a measurement.
        let row = support(&bars, &cand);
        let bmp = vertical.support(&cand);
        if row != bmp {
            println!("  C-E-06 k={k:<2} LAYOUTS DISAGREE — row-major {row}, bitmaps {bmp}");
            ok = false;
            continue;
        }
        let row_ps = once_ps(|| black_box(support(black_box(&bars), black_box(&cand))));
        let bmp_ps = once_ps(|| black_box(vertical.support(black_box(&cand))));
        let times = if bmp_ps == 0 {
            u128::MAX
        } else {
            row_ps / bmp_ps
        };
        let good = times >= FLOOR;
        println!(
            "  {:<58} {:>8} ps -> {:>8} ps   {times}x faster, floor {FLOOR}   {}",
            format!("C-E-06 support at k={k}: row-major -> bitmaps"),
            row_ps,
            bmp_ps,
            if good { "ok" } else { "TOO SLOW" }
        );
        ok &= good;
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
    ok &= the_transposed_column_beats_the_row_major_walk();
    ok &= support_costs_the_same_per_bar_at_every_column_length();
    ok &= support_costs_the_same_per_bar_at_every_depth();
    ok &= support_costs_the_same_whether_bars_match_or_not();
    ok &= a_ladder_walk_costs_the_same_per_bar_at_every_column_length();
    if ok {
        println!("all ratios within the ceiling");
    } else {
        println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
        std::process::exit(1);
    }
}
