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
//! * a miss in **word 0** against a miss in **word 5**, because a branchless
//!   `hits` reads all six words whichever one fails. This pair is evidence
//!   only: an early-exit word loop measured 1.09x to 2.06x across words 1 to 5,
//!   under the 3.0x ceiling, so it passes this row. The guard against that loop
//!   is the source-shape test named below (D-1436);
//! * a **1-bit** candidate against one requiring every live bit (`LIVE`), because the cost must not
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
use vocab::expression::{ENCODED_LEN, Expression, MAX_INSTRUCTIONS, Truth};
use vocab::expression_search::{CURSOR_BYTES, Cursor, Step};
use vocab::mask::WORDS;
use vocab::table::{LIVE, NEXT_FREE, is_live};

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
/// Both directions breach, as in `crates/engine/benches/ratio.rs`. An input that
/// is CHEAPER than the baseline is as much a data dependence as one that is
/// dearer, and a one-sided check passed every such case (D-1436).
///
/// A zero on either side is a FAILURE and not a pass. An operation that timed
/// at zero picoseconds was optimised away, and a ratio against nothing is not a
/// measurement — reporting it as ok is the fallback `CLAUDE.md` §4 bans.
fn ratio(label: &str, base_ps: u128, at_ps: u128) -> bool {
    if base_ps == 0 || at_ps == 0 {
        println!("  {label:<58} UNMEASURABLE — a side timed at zero");
        return false;
    }
    let up = at_ps * 1_000 / base_ps;
    let down = base_ps * 1_000 / at_ps;
    let ok = up.max(down) <= CEILING_PERMILLE;
    println!(
        "  {label:<58} {:>8} ps -> {:>8} ps   ratio {}.{:03}x  {} {}",
        base_ps,
        at_ps,
        up / 1_000,
        up % 1_000,
        if ok { "ok" } else { "BREACH" },
        if down > up { "(cheaper)" } else { "" },
    );
    ok
}

/// The machine's own floor: what [`cost_ps`] reports for the cheapest operation
/// there is, timed by this same loop, in this same build.
///
/// # Why this exists
///
/// Every other measurement in this file is a ratio between two INPUTS to the
/// same operation, and an independent audit measured exactly what that cannot
/// see. A `hits` rewritten as a per-bit probe scan over all [`ConditionMask::BITS`]
/// positions -- 174x slower than the shipped one -- passed C-V-01, C-V-02 and
/// C-V-03 at 0.985x, 1.004x and 0.984x. A cost that slows BOTH legs cancels in a
/// quotient. That is not a loose ceiling or a noisy runner; it is structural, and
/// it holds on any CPU at any ceiling.
///
/// An absolute picosecond ceiling would catch it, and would also measure the
/// runner -- which is the reason this file has ratios at all. This is the third
/// option: a denominator that cannot move when the numerator does.
/// `wrapping_add` on a black-boxed `u64` is one instruction, it is not part of
/// `ConditionMask`, and no change to a mask operation can change it. So the
/// quotient scales with the runner and does NOT scale with a regression.
///
/// The floor is not zero-cost and is not meant to be: it carries this harness's
/// own loop and `black_box` overhead, which every numerator carries too. That
/// makes it the right unit -- "how many of the cheapest thing does this cost".
fn floor_ps() -> u128 {
    cost_ps(200_000, || black_box(1u64).wrapping_add(black_box(1)))
}

/// Prints one budget and returns whether it held.
///
/// Separate from [`ratio`] because the failure means something different. A
/// breached RATIO says the cost depends on the data. A breached BUDGET says the
/// operation got slower for every input at once, which no ratio in this file can
/// report.
fn budget(label: &str, floor: u128, at_ps: u128, allowed: u128) -> bool {
    if floor == 0 {
        println!("  {label:<58} UNMEASURABLE — the floor timed at zero");
        return false;
    }
    let floors = at_ps * 1_000 / floor;
    let ok = floors <= allowed * 1_000;
    println!(
        "  {label:<58} {:>8} ps = {}.{:03} floors, budget {allowed}   {}",
        at_ps,
        floors / 1_000,
        floors % 1_000,
        if ok { "ok" } else { "OVER BUDGET" }
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

/// C-V-06 — condition lookup costs the same wherever in the table it lands.
///
/// # Why this row did not exist until now
///
/// `CLAUDE.md` §3 rule 4 names five operations that must be constant, and
/// **condition lookup is the second of them**. `table::definition` is
/// `TABLE.get(index)` and `table::is_live` wraps it; `engine::Ladder::walk`
/// calls `is_live` once per offered position. An independent O(1) audit of all
/// thirteen crates found that **no bench measured either**.
///
/// The rows above measure the MASK. They say nothing about the TABLE, and the
/// two are different operations on different data — one is six words of
/// register arithmetic, the other is a bounds-checked index into `TABLE`'s
/// rows (370 today) of static memory.
///
/// # Why the index has to vary, and why a miss is included
///
/// A direct index costs the same everywhere; **an early-exit scan does not**.
/// Looking up position 0, the last allocated position (`NEXT_FREE - 1`) and a
/// position past the end is what separates them: an early-exit linear search
/// would be cheap at 0 and dear at the last position, and would walk the
/// whole table before answering the miss. That is the only shape this row can
/// usefully refuse, so it is the shape it measures.
fn a_condition_lookup_costs_the_same_wherever_it_lands() -> bool {
    let live = |i: u16| cost_ps(20_000, || black_box(vocab::table::is_live(black_box(i))));
    let def = |i: u16| cost_ps(20_000, || black_box(vocab::table::definition(black_box(i))));

    // The first row, the last allocated row, and one past the table.
    let last = NEXT_FREE.saturating_sub(1);
    let past = NEXT_FREE.saturating_add(64);

    let base = live(0);
    let mut ok = true;
    ok &= ratio(
        &format!("C-V-06 is_live: position 0 -> position {last}"),
        base,
        live(last),
    );
    ok &= ratio(
        &format!("C-V-06 is_live: position 0 -> position {past}, past the table"),
        base,
        live(past),
    );
    ok &= ratio(
        &format!("C-V-06 is_live: position 0 -> position {}", last / 2),
        base,
        live(last / 2),
    );

    // `definition` is the half `is_live` is built on, measured separately so a
    // regression in the match arm and one in the index cannot hide behind each
    // other.
    let dbase = def(0);
    ok &= ratio(
        &format!("C-V-06 definition: position 0 -> position {last}"),
        dbase,
        def(last),
    );
    ok &= ratio(
        &format!("C-V-06 definition: position 0 -> position {past}, past the table"),
        dbase,
        def(past),
    );
    ok
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
/// A loop returning on the first non-matching word is cheaper on a candidate
/// that fails in word 0 than on one that fails in word 5, so the sweep's cost
/// would depend on which conditions a combination happens to require — a
/// constant-time guarantee that holds only on average, which §3 rule 4 does not
/// accept.
///
/// **This row is not the guard against that loop, and says so.** Audit findings
/// ET-o1-proof-coverage-3 and -13 replaced the body of `hits` with an early-exit
/// word loop and measured 1.093x, 1.275x, 1.560x, 1.895x and 2.056x for words 1
/// to 5: rising with the word, but under [`CEILING_PERMILLE`]. Six words of
/// register arithmetic are too cheap for the exit to cost more than the call
/// around it, and a ceiling tight enough to refuse 2.06x would be inside a
/// shared runner's noise, so Gate 8 would flake. The guard is the source-shape
/// unit test `vocab::mask::hits_does_the_same_work_for_every_input`, which
/// refuses `while`, `for`, `loop`, `return`, `if`, `match`, `&&` and `||` in the
/// body. This row is evidence that the compiled function agrees (D-1436).
/// [`ratio`] breaches in both directions, so a word that became CHEAPER than
/// word 0 fails here too.
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
/// depth parameter, and that is only affordable if evaluating a k=|LIVE|
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

/// C-V-05 — every mask operation costs a bounded multiple of the floor.
///
/// The measurement C-V-01 through C-V-04 structurally cannot take: they compare
/// an operation to ITSELF on a different input, so a uniform slowdown divides
/// out. This compares each operation to something that cannot slow down with it.
///
/// The budget is the measured multiple with headroom for a different
/// microarchitecture, not a round number picked to pass. Measured on an arm64
/// laptop, release, `lto = "fat"`, floor 554 ps: `get` 1.445 floors, `hits`
/// 1.485, `popcount` 1.866, `intersect` 2.007, `union` 2.009, `with_bit` 3.375.
/// Twelve leaves 3.5x over the widest of those and still refuses the 174x
/// regression above by a factor of twenty.
///
/// What this does NOT catch, stated rather than implied: a regression smaller
/// than the headroom. `hits` could become eight times slower and stay inside the
/// budget. The structural half is `vocab::mask::hits_does_the_same_work_for_every_input`,
/// which pins the function line for line, so the shapes that cause a large
/// slowdown -- a loop, a branch, a per-bit scan -- fail the suite before they
/// reach a clock. Recorded in `docs/06-limits.md`.
fn no_operation_costs_more_than_its_budget(floor: u128) -> bool {
    /// Floors allowed per operation. One number, because every operation here is
    /// the same shape -- a fixed number of word operations and no loop over set
    /// bits -- so a per-operation budget would be six copies of one fact.
    const ALLOWED: u128 = 12;

    let bar = every_live_bit();
    let cand = one_bit_in_word(0);
    let full = all_live_bits();

    let mut ok = true;
    for (label, at) in [
        (
            "C-V-05 hits",
            cost_ps(200_000, || black_box(&bar).hits(black_box(&cand))),
        ),
        (
            "C-V-05 popcount",
            cost_ps(200_000, || black_box(&full).popcount()),
        ),
        (
            "C-V-05 union",
            cost_ps(200_000, || black_box(&full).union(black_box(&cand))),
        ),
        (
            "C-V-05 intersect",
            cost_ps(200_000, || black_box(&full).intersect(black_box(&cand))),
        ),
        (
            "C-V-05 with_bit",
            cost_ps(200_000, || {
                black_box(&ConditionMask::ZERO).with_bit(black_box(279))
            }),
        ),
        (
            "C-V-05 get",
            cost_ps(200_000, || black_box(&full).get(black_box(279))),
        ),
    ] {
        ok &= budget(label, floor, at, ALLOWED);
    }
    ok
}

/// Groups per p99 row and samples per group, as in
/// `crates/engine/benches/ratio.rs` (D-4482): every size is measured inside
/// every group, back to back, and the row gates the median of the per-group
/// p99 ratios, so a burst of load must land in five groups of nine to move it.
const P99_GROUPS: usize = 9;
const P99_SAMPLES: usize = 2_000;

/// p50, p99 and max of one group, nanoseconds per sample.
#[derive(Clone, Copy)]
struct Tail {
    p50: u128,
    p99: u128,
    max: u128,
}

/// Times `sample` `P99_SAMPLES` times after `P99_SAMPLES / 5` untimed calls.
fn group_tail(mut sample: impl FnMut() -> Truth) -> Tail {
    let mut ns: Vec<u128> = Vec::with_capacity(P99_SAMPLES);
    for at in 0..P99_SAMPLES + P99_SAMPLES / 5 {
        let start = Instant::now();
        black_box(sample());
        let took = start.elapsed().as_nanos();
        if at >= P99_SAMPLES / 5 {
            ns.push(took);
        }
    }
    ns.sort_unstable();
    let at = |permille: usize| {
        ns.get((ns.len() * permille / 1_000).min(ns.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    };
    Tail {
        p50: at(500),
        p99: at(990),
        max: ns.last().copied().unwrap_or(0),
    }
}

/// `measure(at)` for every program inside every group, rotating which goes
/// first; one vector of [`P99_GROUPS`] tails per program.
fn interleaved(count: usize, mut measure: impl FnMut(usize) -> Tail) -> Vec<Vec<Tail>> {
    let mut out: Vec<Vec<Tail>> = (0..count).map(|_| Vec::with_capacity(P99_GROUPS)).collect();
    for group in 0..P99_GROUPS {
        for step in 0..count {
            let at = (step + group) % count;
            let tail = measure(at);
            if let Some(row) = out.get_mut(at) {
                row.push(tail);
            }
        }
    }
    out
}

/// The median over groups of `field`, scaled by `scale` thousandths.
fn median(tails: &[Tail], field: impl Fn(&Tail) -> u128) -> u128 {
    let mut v: Vec<u128> = tails.iter().map(field).collect();
    v.sort_unstable();
    v.get(v.len() / 2).copied().unwrap_or(0)
}

/// The median of the per-group ratios `at[g] * scale_at / (base[g] * scale_base)`
/// in thousandths, or `None` when a group timed at zero.
fn paired(base: &[Tail], at: &[Tail], scale_base: u128, scale_at: u128) -> Option<u128> {
    if base.len() != at.len() || base.is_empty() {
        return None;
    }
    let mut ratios: Vec<u128> = Vec::with_capacity(base.len());
    for (b, a) in base.iter().zip(at) {
        if b.p99 == 0 || a.p99 == 0 {
            return None;
        }
        ratios.push(a.p99 * scale_at * 1_000 / (b.p99 * scale_base));
    }
    ratios.sort_unstable();
    ratios.get(ratios.len() / 2).copied()
}

/// Two live bit ids, the smaller first, for building canonical programs.
fn two_live_bits() -> (u16, u16) {
    let mut live = (0..u16::try_from(ConditionMask::BITS).unwrap_or(0)).filter(|&b| is_live(b));
    if let (Some(a), Some(b)) = (live.next(), live.next()) {
        return (a, b);
    }
    println!("BENCH SETUP FAILED — fewer than two live bits");
    std::process::exit(1)
}

/// A canonical program from its instructions, through the public decoder:
/// `(opcode, operand)` pairs as `Expression::encode` writes them.
fn decoded(code: &[(u8, u16)]) -> Expression {
    let mut bytes = [0_u8; ENCODED_LEN];
    let count = u16::try_from(code.len()).unwrap_or(0).to_le_bytes();
    for (slot, byte) in bytes.iter_mut().zip([1, 0, count[0], count[1]]) {
        *slot = byte;
    }
    for (chunk, &(op, operand)) in bytes
        .get_mut(4..)
        .unwrap_or_default()
        .chunks_exact_mut(3)
        .zip(code)
    {
        for (slot, byte) in
            chunk
                .iter_mut()
                .zip([op, operand.to_le_bytes()[0], operand.to_le_bytes()[1]])
        {
            *slot = byte;
        }
    }
    match Expression::decode(&bytes) {
        Ok(expression) => expression,
        Err(refusal) => {
            println!("BENCH SETUP FAILED — a bench program was refused: {refusal:?}");
            std::process::exit(1)
        }
    }
}

/// `leaves` leaves joined left to right: `2 * leaves - 1` instructions whose
/// stack never stands taller than two. The first leaf is the smaller bit, so
/// every join's left sibling sorts first and the program is canonical.
fn shallow(leaves: usize) -> Expression {
    let (low, high) = two_live_bits();
    let mut code = vec![(1, low)];
    for _ in 1..leaves {
        code.extend([(1, high), (3, 0)]);
    }
    decoded(&code)
}

/// `leaves` leaves, then every join: the same length as [`shallow`], with a
/// stack `leaves` tall.
fn tallest(leaves: usize) -> Expression {
    let (low, _) = two_live_bits();
    let mut code = vec![(1, low); leaves];
    code.extend(std::iter::repeat_n((3, 0), leaves.saturating_sub(1)));
    decoded(&code)
}

/// FXD-04 and FXD-05 — one expression instruction costs the same at p99
/// whatever the program's length, and a program costs the same whatever its
/// stack height (o1engine-22, D-4484).
///
/// FXD-04: AND chains of 1, 15, 127 and 1,151 instructions. Each sample
/// evaluates the program enough times to run 1,151 instructions, so every
/// sample is the same work; the row divides by instructions and gates growth
/// one-sided, because each call's fixed cost -- the dispatch and the two
/// cleared words -- is amortised over more instructions as the program grows.
/// FXD-05: two programs of 1,151 instructions, one whose stack stands two
/// tall and one 576 tall, gated two-sided: the deep one clears nine words per
/// plane instead of one and must not cost more for it.
fn an_expression_instruction_costs_the_same_at_p99() -> bool {
    let truth = ConditionMask::from_words([u64::MAX; WORDS]);
    let known = truth;
    let programs = vec![
        (shallow(1), "chain, 1 instruction"),
        (shallow(8), "chain, 15 instructions"),
        (shallow(64), "chain, 127 instructions"),
        (shallow(576), "chain, 1,151 instructions"),
        (tallest(576), "height 576, 1,151 instructions"),
    ];
    for (expression, label) in &programs {
        if expression.evaluate(truth, known) != Truth::True {
            println!("BENCH SETUP FAILED — {label} did not evaluate true");
            std::process::exit(1)
        }
    }
    let lens: Vec<u128> = [1_u128, 15, 127, 1_151, 1_151].to_vec();
    let tails = interleaved(programs.len(), |at| {
        let Some((expression, _)) = programs.get(at) else {
            return Tail {
                p50: 0,
                p99: 0,
                max: 0,
            };
        };
        let reps = 1_151 / lens.get(at).copied().unwrap_or(1).max(1);
        group_tail(|| {
            let mut last = Truth::Unknown;
            for _ in 0..reps {
                last = black_box(expression).evaluate(black_box(truth), black_box(known));
            }
            last
        })
    });
    let mut ok = true;
    for (at, ((_, label), groups)) in programs.iter().zip(&tails).enumerate() {
        let len = lens.get(at).copied().unwrap_or(1);
        let reps = 1_151 / len.max(1);
        let instructions = reps * len;
        println!(
            "  FXD-04/05 {label:<32} p50 {:>6} p99 {:>6} max {:>9} ps per instruction",
            median(groups, |t| t.p50) * 1_000 / instructions,
            median(groups, |t| t.p99) * 1_000 / instructions,
            groups.iter().map(|t| t.max).max().unwrap_or(0) * 1_000 / instructions,
        );
    }
    let instructions = |at: usize| {
        let len = lens.get(at).copied().unwrap_or(1);
        (1_151 / len.max(1)) * len
    };
    let Some(base) = tails.first() else {
        return false;
    };
    for at in 1..4 {
        let Some(groups) = tails.get(at) else {
            return false;
        };
        let Some(up) = paired(base, groups, instructions(at), instructions(0)) else {
            println!("  FXD-04 UNMEASURABLE — a group timed at zero");
            return false;
        };
        let held = up <= CEILING_PERMILLE;
        println!(
            "  FXD-04 per-instruction p99, {} -> {} instructions: median ratio {}.{:03}x  {}",
            lens.first().copied().unwrap_or(0),
            lens.get(at).copied().unwrap_or(0),
            up / 1_000,
            up % 1_000,
            if held { "ok" } else { "BREACH" }
        );
        ok &= held;
    }
    let deep = tails
        .get(3)
        .zip(tails.get(4))
        .and_then(|(chain, tall)| paired(chain, tall, 1, 1));
    if let Some(up) = deep {
        let held = up <= CEILING_PERMILLE && 1_000_000 / up.max(1) <= CEILING_PERMILLE;
        println!(
            "  FXD-05 p99, height 2 -> height 576 at 1,151 instructions: median ratio {}.{:03}x  {}",
            up / 1_000,
            up % 1_000,
            if held { "ok" } else { "BREACH" }
        );
        ok &= held;
    } else {
        println!("  FXD-05 UNMEASURABLE — a group timed at zero");
        ok = false;
    }
    ok
}

/// A cursor about to place the last instruction of a 1,151-instruction
/// program: two siblings of 575 instructions each -- a leaf and 574 `Not`s --
/// and the `And` that joins them. With one bit both siblings are the same and
/// the canonical-order check reads all 575 pairs; with two, the right sibling
/// starts with the other bit and the check stops at the first.
fn about_to_join(same: bool) -> Cursor {
    let (low, high) = two_live_bits();
    let alphabet: Vec<u32> = if same {
        vec![u32::from(low)]
    } else {
        vec![u32::from(low), u32::from(high)]
    };
    let count = u16::try_from(alphabet.len()).unwrap_or(1);
    let mut digits = vec![0_u16; MAX_INSTRUCTIONS];
    let sibling = MAX_INSTRUCTIONS / 2;
    for (at, digit) in digits.iter_mut().enumerate().take(MAX_INSTRUCTIONS - 1) {
        let within = at % sibling;
        *digit = if within == 0 {
            // A leaf: rank 0 is the low bit, rank 1 the high one.
            if at == 0 || same { 1 } else { 2 }
        } else {
            // `Not` is the rank after the alphabet; a digit is rank + 1.
            count + 1
        };
    }
    // The position being placed: the next rank to try is `And`.
    if let Some(last) = digits.last_mut() {
        *last = count + 1;
    }
    let mut bytes = [0_u8; CURSOR_BYTES];
    let length = u16::try_from(MAX_INSTRUCTIONS).unwrap_or(0);
    let at = length - 1;
    let header = b"BTXEGN01"
        .iter()
        .copied()
        .chain(count.to_le_bytes())
        .chain(length.to_le_bytes())
        .chain(at.to_le_bytes())
        .chain([0, 0]);
    let offered = (0..384).flat_map(|i| {
        u16::try_from(alphabet.get(i).copied().unwrap_or(0))
            .unwrap_or(0)
            .to_le_bytes()
    });
    for (slot, byte) in bytes.iter_mut().zip(
        header
            .chain(offered)
            .chain(digits.iter().flat_map(|d| d.to_le_bytes())),
    ) {
        *slot = byte;
    }
    match Cursor::decode(&bytes) {
        Ok(cursor) => cursor,
        Err(refusal) => {
            println!("BENCH SETUP FAILED — the crafted cursor was refused: {refusal:?}");
            std::process::exit(1)
        }
    }
}

/// FXD-06 — one search node's sibling-order check is bounded by the program
/// it is placing, and at its worst costs no more than three times the same
/// placement where the siblings differ at once (o1engine-23, D-4485).
///
/// Both placements finish a 1,151-instruction program and emit it, so both
/// pay the candidate's fixed-width copy and height measurement; the only
/// difference is the 575-pair comparison the identical siblings force. A
/// realistic search -- three bits, from the start -- is printed beside it.
fn a_search_node_is_bounded_at_p99() -> bool {
    let joins: Vec<Cursor> = [false, true].into_iter().map(about_to_join).collect();
    let tails = interleaved(2, |at| {
        let Some(start) = joins.get(at) else {
            return Tail {
                p50: 0,
                p99: 0,
                max: 0,
            };
        };
        let mut cursor = start.clone();
        let mut work = 0_u64;
        let mut ns: Vec<u128> = Vec::with_capacity(P99_SAMPLES);
        for sample in 0..P99_SAMPLES + P99_SAMPLES / 5 {
            cursor.clone_from(start);
            let begin = Instant::now();
            let step = cursor.advance(1, &mut work);
            let took = begin.elapsed().as_nanos();
            if !matches!(step, Ok(Step::Candidate(_))) {
                println!("BENCH SETUP FAILED — the crafted placement emitted no candidate");
                std::process::exit(1)
            }
            black_box(step.ok());
            if sample >= P99_SAMPLES / 5 {
                ns.push(took);
            }
        }
        ns.sort_unstable();
        let rank = |permille: usize| ns.get(ns.len() * permille / 1_000).copied().unwrap_or(0);
        Tail {
            p50: rank(500),
            p99: rank(990),
            max: ns.last().copied().unwrap_or(0),
        }
    });
    let mut ok = true;
    for (label, groups) in ["siblings differ at once", "siblings identical, 575 pairs"]
        .iter()
        .zip(&tails)
    {
        println!(
            "  FXD-06 {label:<32} p50 {:>7} p99 {:>7} max {:>9} ns per node",
            median(groups, |t| t.p50),
            median(groups, |t| t.p99),
            groups.iter().map(|t| t.max).max().unwrap_or(0),
        );
    }
    let node = tails
        .first()
        .zip(tails.get(1))
        .and_then(|(differ, identical)| paired(differ, identical, 1, 1));
    if let Some(up) = node {
        let held = up <= CEILING_PERMILLE;
        println!(
            "  FXD-06 p99, siblings differ -> identical: median ratio {}.{:03}x  {}",
            up / 1_000,
            up % 1_000,
            if held { "ok" } else { "BREACH" }
        );
        ok &= held;
    } else {
        println!("  FXD-06 UNMEASURABLE — a group timed at zero");
        ok = false;
    }

    // A realistic search: three live bits from the start, one node a sample.
    let (low, high) = two_live_bits();
    let third = (high + 1..u16::try_from(ConditionMask::BITS).unwrap_or(0))
        .find(|&b| is_live(b))
        .unwrap_or(high);
    if let Ok(mut cursor) = Cursor::new(&[u32::from(low), u32::from(high), u32::from(third)]) {
        let mut work = 0_u64;
        let mut ns: Vec<u128> = Vec::with_capacity(200_000);
        let mut candidates = 0_u64;
        for _ in 0..200_000 {
            let begin = Instant::now();
            let step = cursor.advance(1, &mut work);
            ns.push(begin.elapsed().as_nanos());
            if matches!(step, Ok(Step::Candidate(_))) {
                candidates += 1;
            }
            black_box(step.ok());
        }
        ns.sort_unstable();
        let rank = |permille: usize| ns.get(ns.len() * permille / 1_000).copied().unwrap_or(0);
        println!(
            "  FXD-06 realistic search, 3 bits, 200,000 nodes ({candidates} candidates): \
             p50 {} p99 {} max {} ns per node — REPORTED, not gated",
            rank(500),
            rank(990),
            ns.last().copied().unwrap_or(0)
        );
    }
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
    let floor = floor_ps();
    println!("  the machine's floor is {floor} ps — one black-boxed wrapping_add");
    let mut ok = true;
    ok &= no_operation_costs_more_than_its_budget(floor);
    ok &= a_hit_and_a_miss_cost_the_same();
    ok &= a_condition_lookup_costs_the_same_wherever_it_lands();
    ok &= a_miss_costs_the_same_in_every_word();
    ok &= the_cost_does_not_grow_with_what_the_candidate_requires();
    ok &= the_set_operations_do_not_grow_with_the_bits_set();
    ok &= an_expression_instruction_costs_the_same_at_p99();
    ok &= a_search_node_is_bounded_at_p99();
    if ok {
        println!("all ratios within the ceiling");
    } else {
        println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
        std::process::exit(1);
    }
}
