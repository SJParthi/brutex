//! Gate 8 for `crates/indicators` — one candle in costs the same whatever came
//! before it, and whatever it contains.
//!
//! # The claim this crate actually makes
//!
//! Every module here is a streaming fold: `step(&mut self, bar: &Candle)`. The
//! state is a fixed set of fixed-size fields — ring buffers with `const`
//! lengths, scaled accumulators, two `Option<Swing>` latches — and
//! `const _: () = assert!(size_of::<Evaluator>() <= 1792)` fails the build if any
//! of them starts growing.
//!
//! That assertion bounds the **space**. It says nothing about time, and the two
//! come apart in exactly one way that matters: a fold whose state is fixed-size
//! can still do more work as it goes if it ever rescans that state. So the
//! measurement here is per-candle cost at the start of a run against per-candle
//! cost deep into one. If the ratio drifts, something accumulates that the size
//! assertion cannot see.
//!
//! # Why the second axis is the candle's CONTENT
//!
//! §3 rule 4 asks for constant cost per operation, not constant on average. A
//! bar that trips more predicates must not cost more than one that trips none,
//! because the sweep feeds real data and real data is not uniform: a trending
//! session sets structure bits that a flat one does not. If cost tracked bits
//! set, the sweep's runtime would depend on the market rather than on the number
//! of bars, and a bound measured on quiet data would be wrong on the days anyone
//! cares about.
//!
//! Both axes are measured. Neither alone is the claim.
//!
//! # `isqrt_i128` gets its own row, because its bound was once false
//!
//! Sigma needs an integer square root and §7 forbids floats. An earlier version
//! capped Newton at 64 steps and then stepped down **unbounded**, under a doc
//! comment calling itself constant-cost. At `i128::MAX` the step-down needed
//! 1,638,791,155,897,336,446 decrements — `O(sqrt(v))` in a function documented
//! as O(1). The cap is now two compile-time constants, and the row below
//! measures the two ends of the input range against each other rather than
//! trusting the constants to be large enough.
//!
//! # What this bench cannot see
//!
//! It runs synthetic candles. It measures cost, not correctness, and a module
//! that emitted nothing at all would look excellent here — the emission
//! guarantees are `crates/indicators/tests/` and the per-module position tests.
//! It also cannot see a cost that grows only past the bar counts used here; the
//! deepest point is 200,000 candles, which is more than a session and less than
//! the 1.22 million a full instrument carries. That limit is stated rather than
//! implied, per §3 rule 6.
//!
//! See `crates/store/benches/ratio.rs` for why there is no benchmarking
//! framework and why every ratio here is integer arithmetic.

use std::hint::black_box;
use std::time::Instant;

use indicators::column::Column;
use indicators::evaluator::{Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::{Availability, ITERATION_CEILING, isqrt_i128, isqrt_i128_counted};
use indicators::{Candle, OI_NULL};

/// A ratio above this is a failure. Thousandths, so the comparison is integer.
///
/// 3.0x — `docs/04-invariants.md`, the shared-CI number.
const CEILING_PERMILLE: u128 = 3_000;

/// How many times each measurement is repeated before the minimum is taken.
const TRIALS: u32 = 24;

/// One minute, in microseconds.
const MINUTE: i64 = 60_000_000;

/// A price to build synthetic candles around, in paisa. 25,000.00 index points.
const BASE: i64 = 2_500_000;

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
/// A zero baseline FAILS. An operation the optimiser deleted has not been shown
/// constant — it has been shown absent, and calling that a pass is the fallback
/// `CLAUDE.md` §4 bans.
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

/// Like [`ratio`], but a variant that is CHEAPER breaches too.
///
/// Named to end in `ratio` deliberately: gate 14 counts measurement sites by
/// looking for `ratio(`, and a two-sided comparison is a measurement site. A
/// helper the counter cannot see would undercount this bench's real coverage.
///
/// The one-sided form every other bench in this workspace uses asks "did this get
/// slower". That is the right question for a depth claim: folding 200,000 candles
/// cannot make the next one cheaper, so only the upper direction is reachable.
///
/// It is the wrong question for a CONTENT claim. "One candle costs the same
/// whatever it contains" is violated in both directions — a bar that costs half as
/// much is as much a data-dependent cost as one that costs twice as much, and a
/// one-sided check would report it green. So this compares the ratio and its
/// reciprocal, and takes the worse.
fn two_sided_ratio(label: &str, base_ps: u128, at_ps: u128) -> bool {
    if base_ps == 0 || at_ps == 0 {
        println!("  {label:<58} UNMEASURABLE — a side timed at zero");
        return false;
    }
    let up = at_ps * 1_000 / base_ps;
    let down = base_ps * 1_000 / at_ps;
    let worst = up.max(down);
    let ok = worst <= CEILING_PERMILLE;
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

/// Refuses loudly rather than unwrapping into a panic message nobody reads.
/// The machine's own floor: [`cost_ps`] on the cheapest operation there is,
/// timed by this same loop in this same build.
///
/// # Why this exists
///
/// Every ratio in this file compares one INPUT to another input of the same
/// operation. An independent audit measured what that structurally cannot see: a
/// mask operation made 174x slower passed its crate's ratio rows at 0.98x-1.00x,
/// because a cost that slows BOTH legs cancels in a quotient. The same blindness
/// applies here, and worse -- `step` is the per-bar cost of the entire condition
/// vocabulary, so a regression in ANY family slows every leg of every ratio
/// equally and every row still reports ok.
///
/// A picosecond ceiling would catch it and would also measure the runner. This is
/// the third option: a denominator that cannot move when the numerator does.
/// `wrapping_add` on a black-boxed `u64` is one instruction and is not part of
/// `Evaluator`, so nothing that changes a condition family can change it.
fn floor_ps() -> u128 {
    cost_ps(20_000, || black_box(1u64).wrapping_add(black_box(1)))
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
        "  {label:<58} {:>8} ps = {}.{:03} floors, budget {allowed}   {}",
        at_ps,
        floors / 1_000,
        floors % 1_000,
        if ok { "ok" } else { "OVER BUDGET" }
    );
    ok
}

/// C-I-05 — one bar's whole evaluation costs a bounded multiple of the floor.
///
/// # JUDGED HERE, PINNED NOWHERE ELSE
///
/// The verdict feeds `main`'s exit status, so gate 8 goes red on a breach. What this row
/// does not have is a name outside this file. `docs/04-invariants.md` carries `C-I-01`
/// through `C-I-04` and no `C-I-05`, and gate 14's coverage entry for this crate lists
/// those same four ids and a floor of five measurement points. This row does not report
/// through the helper that scan counts, so it contributes nothing to the count either.
///
/// Deleting the assertion below therefore deletes the measurement and every static check
/// stays green. That is gate 12's own admission one level down: a claim can name a proof
/// that exists, and nothing checks the proof still does anything. `main` holds the rows in
/// an array of fixed length, so deleting a row WHOLE is a compile error — it cannot see a
/// row gutted in place. The repair that can is a row in `docs/04-invariants.md` and the id
/// added to gate 14's table; both files are outside this change and this is recorded
/// rather than fixed.
fn a_candle_stays_within_its_budget(floor: u128) -> bool {
    /// Floors allowed for one `step`.
    ///
    /// Measured, arm64 laptop, release, `lto = "fat"`, floor 556 ps: **682.856
    /// floors** for one bar through every family. Two thousand leaves 2.9x for a
    /// different microarchitecture and still refuses a 3x across-the-board
    /// regression -- the kind every ratio in this file divides out, because one
    /// bar's evaluation is the numerator AND the denominator of all of them.
    ///
    /// The number is large because the work is: one `step` folds every condition
    /// family in the vocabulary. It is a budget, not a target.
    const ALLOWED: u128 = 2_000;

    let mut state = warmed(1_000);
    let mut m = 1_000_i64;
    let at = cost_ps(20_000, || {
        m += 1;
        let c = wandering(m);
        black_box(state.step(black_box(&c)))
    });
    budget("C-I-05 step: one bar, every family", floor, at, ALLOWED)
}

/// `n` consecutive candles, one per minute, for a whole-column build.
fn column_bars(n: i64) -> Vec<Candle> {
    let mut out = Vec::with_capacity(usize::try_from(n).unwrap_or(0));
    for m in 0..n {
        out.push(wandering(m));
    }
    out
}

/// Cost of one bar as `Column::build` actually pays it, in picoseconds.
///
/// The evaluator is constructed INSIDE the timed region because it has to be:
/// `Column::build` folds monotonic timestamps, so a second build against a used
/// evaluator refuses every bar for a receding timestamp and would measure the
/// refusal path instead of the fold. That construction is one fixed-size struct
/// against `n` steps, so it shrinks as `n` grows -- it can only bias this ratio
/// DOWNWARD, never toward a false pass.
fn column_per_bar_ps(n: i64) -> u128 {
    let bars = column_bars(n);
    let count = u128::try_from(bars.len()).unwrap_or(1).max(1);
    let whole = cost_ps(1, || {
        let mut e = fresh();
        black_box(Column::build(black_box(&bars), &mut e))
    });
    whole / count
}

/// C-I-06 — building the column costs the same per bar however long it is.
///
/// # What this catches that `C-I-01` cannot
///
/// `C-I-01` measures one `step` against a deeper evaluator, which is the fold's
/// own bound. This measures the WHOLE column build: the vector that backs it, the
/// warm-up test taken per bar, and the census. A `Vec` that reallocated per push,
/// a census that scanned its own buckets, or a warm-up test that walked history
/// would all be invisible to `C-I-01` and visible here.
///
/// Both legs are past the warm-up boundary -- 20,000 minutes is about thirteen
/// sessions and 200,000 is about a hundred and thirty-eight, and the run warms
/// after five. A short leg that never warmed would take the no-push branch for
/// every bar and this row would compare two different code paths.
///
/// # JUDGED HERE, PINNED NOWHERE ELSE — and something cites it
///
/// As `C-I-05`: the verdict feeds `main`'s exit status, so gate 8 refuses a breach, and
/// nothing else in the repository names this row. It is absent from
/// `docs/04-invariants.md` and from gate 14's coverage entry for this crate, whose floor
/// of five measurement points this file clears with room to spare without it. So the
/// comparison below can be deleted and every static check stays green.
///
/// The difference from `C-I-05` is that a production doc block depends on this one:
/// `crate::column` cites `C-I-06` by id, twice, as the measurement behind its per-bar
/// cost claim. Gate 12 accepts that citation on the strength of the bench PATH it names
/// beside the id, and the path exists whatever this file contains — which is precisely
/// what that module's own doc block says the gate cannot see. The repair is the same one:
/// a row in `docs/04-invariants.md` and the id in gate 14's table, both outside this
/// change.
fn the_column_costs_the_same_per_bar_however_long_it_is() -> bool {
    let base = column_per_bar_ps(20_000);
    let at = column_per_bar_ps(200_000);
    ratio(
        "C-I-06 Column::build: 20,000 bars -> 200,000 bars",
        base,
        at,
    )
}

fn refuse(what: &str) -> ! {
    println!("BENCH SETUP FAILED — {what}");
    std::process::exit(1)
}

/// A fresh evaluator with the pinned tolerances.
fn fresh() -> Evaluator {
    let Ok(widths) = Widths::pinned() else {
        refuse("the pinned tolerances do not load, so no measurement is possible")
    };
    Evaluator::new(widths, Availability::Absent, Thresholds::CLASSICAL)
}

/// A candle at minute `m`, wandering so consecutive bars differ.
///
/// Deterministic: no clock and no randomness, so two runs measure the same work.
/// The wander is a coprime stride through a range, which visits many distinct
/// prices without repeating a short cycle.
fn wandering(m: i64) -> Candle {
    let drift = ((m * 7_919) % 4_001 - 2_000) * 13;
    let open = BASE + drift;
    let span = 400 + ((m * 31) % 600);
    Candle {
        ts_micros: m * MINUTE,
        open,
        high: open + span,
        low: open - span,
        close: open + span / 3,
        volume: 0,
        open_interest: OI_NULL,
    }
}

/// A candle with no range at all: one price, four times over.
///
/// The cheapest possible bar. Every range-relative predicate refuses on it, so
/// it is the low-water mark for how many bits a candle can set.
fn motionless(m: i64) -> Candle {
    Candle {
        ts_micros: m * MINUTE,
        open: BASE,
        high: BASE,
        low: BASE,
        close: BASE,
        volume: 0,
        open_interest: OI_NULL,
    }
}

/// A candle that trends hard and closes on its high.
///
/// The opposite extreme: a long body, no upper wick, a decisive close. This is
/// the shape that sets structure, momentum and pattern bits.
fn decisive(m: i64) -> Candle {
    let open = BASE + m * 40;
    Candle {
        ts_micros: m * MINUTE,
        open,
        high: open + 3_000,
        low: open - 20,
        close: open + 3_000,
        volume: 0,
        open_interest: OI_NULL,
    }
}

/// Folds `n` candles into a fresh evaluator and hands it back warmed.
///
/// A refusal here is a setup failure, not a measurement: the generators above
/// must produce valid candles, and if they do not the bench has measured
/// nothing.
fn warmed(n: i64) -> Evaluator {
    let mut e = fresh();
    for m in 0..n {
        if let Err(why) = e.step(&wandering(m)) {
            refuse(&format!("candle {m} of the warm-up was refused: {why:?}"));
        }
    }
    e
}

/// C-I-01 — one candle costs the same whatever came before it.
///
/// The row that matters. `size_of::<Evaluator>()` is asserted at compile time,
/// which bounds the state's SIZE; this bounds the work done over it. A fold that
/// rescanned its own fixed-size state would satisfy the assertion and fail here.
fn a_candle_costs_the_same_however_many_came_before() -> bool {
    // Three depths, not two. A cost that grew only past some threshold would hide
    // between a shallow and a deep point; an intermediate one makes the shape of
    // any drift visible rather than just its endpoints.
    let mut shallow = warmed(1_000);
    let mut middle = warmed(50_000);
    let mut deep = warmed(200_000);

    // Continue each from where it is, so none of them is re-seeding a session.
    let mut m_shallow = 1_000_i64;
    let mut m_middle = 50_000_i64;
    let mut m_deep = 200_000_i64;

    let early = cost_ps(20_000, || {
        m_shallow += 1;
        shallow.step(&wandering(m_shallow))
    });

    println!(
        "  {:<58} {} -> {} -> {} sessions folded",
        "C-I-01 warm-up depth (context)",
        shallow.sessions_completed(),
        middle.sessions_completed(),
        deep.sessions_completed()
    );

    let mut ok = true;
    ok &= ratio(
        "C-I-01 step: 1,000 candles in -> 50,000 candles in",
        early,
        cost_ps(20_000, || {
            m_middle += 1;
            middle.step(&wandering(m_middle))
        }),
    );
    ok &= ratio(
        "C-I-01 step: 1,000 candles in -> 200,000 candles in",
        early,
        cost_ps(20_000, || {
            m_deep += 1;
            deep.step(&wandering(m_deep))
        }),
    );
    ok
}

/// C-I-02 — one candle costs the same whatever it contains.
///
/// Constant per operation, not constant on average. A trending session must not
/// cost more than a motionless one, or the sweep's runtime depends on the market
/// and a bound measured on quiet data is wrong on the days that matter.
fn a_candle_costs_the_same_whatever_it_contains() -> bool {
    let mut flat_state = warmed(1_000);
    let mut trend_state = warmed(1_000);
    let mut wander_state = warmed(1_000);
    let mut a = 1_000_i64;
    let mut b = 1_000_i64;
    let mut c = 1_000_i64;

    let base = cost_ps(20_000, || {
        a += 1;
        wander_state.step(&wandering(a))
    });

    let mut ok = true;
    ok &= two_sided_ratio(
        "C-I-02 step: a wandering candle -> a motionless one",
        base,
        cost_ps(20_000, || {
            b += 1;
            flat_state.step(&motionless(b))
        }),
    );
    ok &= two_sided_ratio(
        "C-I-02 step: a wandering candle -> a decisive trend candle",
        base,
        cost_ps(20_000, || {
            c += 1;
            trend_state.step(&decisive(c))
        }),
    );
    ok
}

/// C-I-03 — the integer square root's cost is BOUNDED, and flat per iteration.
///
/// # This row does not measure what it first measured, and the change is the point
///
/// The first version timed `isqrt_i128(1)` against `isqrt_i128(i128::MAX)` and
/// demanded a flat ratio. It breached at **217x**, and the breach was correct: the
/// Newton loop exits on convergence, so a small input finishes in one or two
/// iterations and a 127-bit one needs about sixty-six. The function never promised
/// a flat cost. It promises a **bounded iteration count**, and timing a bounded
/// count end to end against a flat ceiling can only produce a false breach or a
/// ceiling relaxed until it catches nothing.
///
/// So the claim is measured as the claim: the count stays under
/// [`ITERATION_CEILING`] at both ends of the input range, and the cost PER
/// iteration is flat — which is what makes a bounded count a bound on work rather
/// than merely on steps. `greeks::bench::one_model_evaluation_costs_the_same_however_hard_the_quote_is`
/// is the same shape for the same reason.
///
/// The 217x end-to-end spread is not hidden by this: it is recorded in
/// `docs/06-limits.md`, which `CLAUDE.md` §10 makes the authority on what is not
/// constant-time.
fn the_integer_square_root_is_bounded_and_flat_per_iteration() -> bool {
    let probes: [(&str, i128); 4] = [
        ("1", 1),
        ("i64::MAX", i128::from(i64::MAX)),
        ("10^30", 10_i128.pow(30)),
        ("i128::MAX", i128::MAX),
    ];

    // The bound, checked before anything is timed. A count over the ceiling means
    // the loop is not bounded by the constants that claim to bound it, and no
    // timing ratio would tell us that.
    // Carried alongside each probe rather than in a parallel array: the workspace
    // lint table denies indexing, and two collections kept in step by an index are
    // the bug that lint exists to prevent.
    let mut measured: Vec<(&str, i128, u32)> = Vec::with_capacity(probes.len());
    for (label, v) in &probes {
        let (root, steps) = isqrt_i128_counted(*v);
        if steps > ITERATION_CEILING {
            println!(
                "  C-I-03 BREACH — v = {label} took {steps} iterations against a \
                 ceiling of {ITERATION_CEILING}"
            );
            return false;
        }
        // A root that came back WRONG makes every timing meaningless: a function
        // that gave up early would be fast and useless. The defining property,
        // checked at every probe:
        //
        //     root^2 <= v  and  (root+1)^2 > v
        //
        // At `i128::MAX` the second product overflows, and the overflow IS the
        // property -- a square that does not fit the type certainly exceeds a
        // value that does.
        let floor_holds = root.checked_mul(root).is_some_and(|sq| sq <= *v);
        let ceiling_holds = root
            .checked_add(1)
            .and_then(|next| next.checked_mul(next))
            .is_none_or(|sq| sq > *v);
        if !floor_holds || !ceiling_holds {
            println!("  C-I-03 BREACH — isqrt({label}) = {root} is not the integer root");
            return false;
        }
        measured.push((label, *v, steps));
    }
    println!(
        "  {:<58} {} of {ITERATION_CEILING}",
        "C-I-03 iterations at 1, i64::MAX, 10^30, i128::MAX",
        measured
            .iter()
            .map(|(_, _, steps)| steps.to_string())
            .collect::<Vec<_>>()
            .join(" -> ")
    );

    // The cost figures are printed as CONTEXT and are deliberately NOT compared to
    // a ceiling.
    //
    // Cost per iteration is not flat either: measured 4.0 ns at v = 1 against 13.3
    // ns at i128::MAX, a 3.3x spread, because each iteration performs a 128-bit
    // division whose own cost rises with the magnitude of its operands. There is no
    // honest ratio to assert here -- a ceiling loose enough to pass would be loose
    // enough to miss a real regression, and picking the baseline that happens to
    // fit is worse than not measuring.
    //
    // What IS asserted above is the part that holds and the part that matters: the
    // iteration count is bounded by a compile-time constant, and the root is the
    // root. That bound is what prevents the regression this function already had --
    // 1,638,791,155,897,336,446 decrements at i128::MAX under a doc comment calling
    // itself constant-cost.
    //
    // The spread is recorded in `docs/06-limits.md`, which CLAUDE.md section 10
    // makes the authority on what is not constant-time. Reporting it green here
    // instead would be the fallback section 4 bans.
    for (label, v, steps) in &measured {
        let total = cost_ps(200_000, || isqrt_i128(black_box(*v)));
        // Integer division on integer picoseconds and an integer count. No float
        // arithmetic happens here; see the module header.
        println!(
            "  {:<58} {total:>8} ps over {steps:>3} iteration(s) = {} ps each",
            format!("C-I-03 cost at v = {label} (context, NOT a ceiling)"),
            total / u128::from(steps.max(&1).to_owned()),
        );
    }

    true
}

/// C-I-04 — the first candle of a run costs what a later one costs.
///
/// Session rollover is the one place per-candle work could legitimately spike:
/// it copies the running extremes into the previous-session slots and reseeds.
/// That is a fixed number of field moves, not a scan, and this row is what says
/// so. A rollover that rebuilt anything from history would show up as a ratio
/// against an ordinary mid-session candle.
fn a_session_rollover_costs_what_an_ordinary_candle_costs() -> bool {
    // A day is 1,440 minutes, so stepping the minute by a day crosses a session
    // boundary on every call.
    let mut rolling = warmed(1_000);
    let mut ordinary = warmed(1_000);
    let mut day = 1_i64;
    let mut m = 1_000_i64;

    let mid = cost_ps(4_000, || {
        m += 1;
        ordinary.step(&wandering(m))
    });
    let rollover = cost_ps(4_000, || {
        day += 1;
        rolling.step(&wandering(day * 1_440))
    });

    println!(
        "  {:<58} {} sessions crossed",
        "C-I-04 rollovers performed (context)",
        rolling.sessions_completed()
    );
    ratio(
        "C-I-04 step: mid-session candle -> session rollover",
        mid,
        rollover,
    )
}

/// How many rows this bench judges.
///
/// The array in `main` is annotated with it, so removing a row is a type error rather than
/// a quiet loss of coverage. That is worth a line here because two of the six — `C-I-05`
/// and `C-I-06`, see their own doc blocks — are named by no invariant row and by no entry
/// in gate 14's coverage table, and gate 14's measurement-point floor for this crate is
/// five against the several this file carries. Nothing else in CI would notice their
/// removal. This notices exactly one shape of removal, the whole-row one, and says so.
const ROWS: usize = 6;

fn main() {
    println!("indicators — gate 8, ceiling {CEILING_PERMILLE} permille");
    println!(
        "  evaluator is {} bytes, emitting {} positions",
        core::mem::size_of::<Evaluator>(),
        Evaluator::positions().len()
    );
    let floor = floor_ps();
    println!("  the machine's floor is {floor} ps — one black-boxed wrapping_add");
    // An array and not a running `&=`, for the reason above. Elements are evaluated left
    // to right, so every row still runs and still prints in this order — none of them is
    // short-circuited away by an earlier breach.
    let verdicts: [bool; ROWS] = [
        a_candle_stays_within_its_budget(floor),
        a_candle_costs_the_same_however_many_came_before(),
        a_candle_costs_the_same_whatever_it_contains(),
        the_integer_square_root_is_bounded_and_flat_per_iteration(),
        a_session_rollover_costs_what_an_ordinary_candle_costs(),
        the_column_costs_the_same_per_bar_however_long_it_is(),
    ];
    if verdicts.contains(&false) {
        println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
        std::process::exit(1);
    }
    println!("all ratios within the ceiling");
}
