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

fn main() {
    println!("engine — gate 8, ceiling {CEILING_PERMILLE} permille");
    println!(
        "  ladder is {} byte(s) — no room for a depth field; mask is {} bits",
        core::mem::size_of::<Ladder>(),
        ConditionMask::BITS
    );
    let mut ok = true;
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
