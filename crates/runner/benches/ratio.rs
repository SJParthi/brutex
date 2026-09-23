//! Gate 8 for `crates/runner` — the three bounds this crate's headers claim,
//! re-measured instead of argued.
//!
//! # Why this file exists at all
//!
//! `crates/runner` shipped with three doc blocks claiming a cost and no bench.
//! Its own `lib.rs` header said so out loud — "UNVERIFIED as a measured figure:
//! this crate ships no bench yet" — which satisfies gate 12, whose question is
//! whether a claim names its proof, and does **not** satisfy gate 14, whose
//! question is whether the crate re-measures the bound it promises. An
//! adversarial audit ran gate 14's own script and found the crate refused.
//! Admitting a number is missing is not the same as taking it.
//!
//! # What is measured, and what deliberately is not
//!
//! Every row divides one per-unit cost by another per-unit cost of the **same**
//! operation, and compares the quotient against [`CEILING_PERMILLE`]. An
//! absolute nanosecond figure measures the machine; a ratio is what survives
//! moving between machines.
//!
//! Three bounds, and each is a claim some header in this crate makes:
//!
//! * **C-R-01** — the **column build** costs the same per offered bar at every
//!   column length. `lib.rs` says the per-bar work is one `Column` step; if the
//!   constant in front of that drifted with the column, a 1.2-million-bar
//!   instrument would cost more than 120 times what 10,000 bars do. The LADDER
//!   is deliberately excluded — it is O(candidates), `engine`'s bench bounds it,
//!   and this row would be measuring the vocabulary if it were included.
//! * **C-R-02** — the audit costs **nothing per bar**. `report.rs` says it
//!   "costs one string per run rather than anything per bar", which is the whole
//!   reason it is allowed to exist outside gate 17's silence. Both ladders are
//!   neutered to one level so only the column varies, and a render whose cost
//!   tracked the column would make that sentence false.
//! * **C-R-03** — assembling a run identity does not depend on how many bars
//!   the digest covered. By the time `identity()` runs, the data digest is
//!   thirty-two bytes whatever produced it; hashing eight fixed-width terms
//!   cannot see the column behind one of them.
//!
//! Two budget rows sit beside them. **C-R-04** prices the index column build in
//! per-bar floors. **C-R-05** prices the column build a cash equity runs, with
//! `Availability::Present` turning the VWAP family on, against a floor timed
//! for long enough to hold still. D-0690.
//!
//! **Not measured here:** whether any of those costs is *small*. This file
//! refuses a cost that GROWS. `docs/06-limits.md` is where absolute figures and
//! the things nobody has timed are recorded.

use core::hint::black_box;
use std::time::Instant;

use brutex_core::instrument::{Exchange, InstrumentKey};
use engine::Ladder;
use indicators::column::Column;
use indicators::evaluator::{Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::{Availability, Vwap};
use runner::identity::{Direction, Params, Run, data_digest, identity};
use runner::{Sweeper, report, synthetic};
use vocab::ConditionMask;

/// How far a ratio may sit from 1.000 before the row is a breach.
///
/// 2.500x. Wider than the engine's rows because every measurement here contains
/// a whole sweep or a whole render rather than one arithmetic operation, so the
/// noise floor is correspondingly higher — and a bound this loose still refuses
/// the thing it exists to refuse. A per-bar cost that grew with the column would
/// not land at 2.4x; it would land at 10x and upward, which is the shape of the
/// regression, not a near miss.
const CEILING_PERMILLE: u128 = 2_500;

/// Repetitions for a site that costs milliseconds — a whole sweep or render.
const REPS: u32 = 5;

/// Repetitions for a site that costs about a microsecond.
///
/// `identity` hashes eight fixed-width terms and returns. At `REPS` the first
/// measurement of it read 1,083,200 ps against 400,000 ps for the same work on
/// a different digest — a 0.369x "improvement" that was the scheduler and a cold
/// instruction cache, not the function. Five repetitions of a microsecond is
/// five microseconds, which is below the noise floor of anything this timer can
/// see.
const REPS_FAST: u32 = 20_000;

/// The support FRACTION every timed sweep runs at, in permille.
///
/// Holding `min_hits` constant across two column lengths does not compare like
/// with like: at 600 hits, a short column demands more hits than it has bars and
/// the ladder dies at k=1, while a long one is a 27% threshold that climbs to
/// depth 12. The first measurement read that as a per-bar cost growing 10.9x. It
/// was two different amounts of work.
///
/// THE FIXTURE LENGTHS ARE ALSO LOAD-BEARING, and the first pair was wrong. The
/// warm-up prefix is a fixed 1,876 bars, so `sessions(4)` offers 1,500 and
/// sweeps NONE of them: the per-bar divisor fell to one and C-R-01 read
/// 442,350,000 ps/bar, which is not a cost but a division. 8 and 32 sessions
/// sweep 1,124 and 10,124 bars -- a 9x range, both warm.
///
/// A fixed fraction makes both columns walk a ladder of comparable shape, which
/// is the only condition under which the quotient says anything about cost per
/// bar. The ceiling was NOT widened to accommodate the artifact — widening a
/// bound until the measurement passes is how a gate stops meaning anything.
const SUPPORT_PERMILLE: u64 = 500;

/// Passes over the fixture per trial when timing the STABLE per-bar floor.
///
/// C-R-04's floor is five passes over 3,000 bars, about 10 µs, timed once: on
/// CI it read 309 ps on one runner and 643 ps on another (D-0680). Two thousand
/// passes is six million adds per trial, milliseconds rather than microseconds,
/// and [`TRIALS`] of them are taken so the minimum can be kept. D-0690.
const FLOOR_PASSES: u32 = 2_000;

/// Whole sweeps per trial for a stable budget row's numerator.
const BUILD_REPS: u32 = 10;

/// Trials per stable measurement. The MINIMUM is kept: an interrupt, a
/// migration to an efficiency core or a clock still ramping can only make a
/// trial slower, so the fastest trial is the one nearest the code's own cost.
const TRIALS: u32 = 15;

fn evaluator() -> Evaluator {
    evaluator_with(Availability::Absent)
}

/// A fresh evaluator whose VWAP family is on or off by `availability`.
///
/// `Present` is what `cli`'s `stored::vwap_availability` binds for a cash
/// equity. Every index run, and every other row in this file, uses `Absent`.
fn evaluator_with(availability: Availability) -> Evaluator {
    Evaluator::new(
        Widths::pinned().unwrap_or_else(|_| {
            eprintln!("the pinned widths are invalid, which is a defect in vocab");
            std::process::exit(2);
        }),
        availability,
        Thresholds::CLASSICAL,
    )
}

/// A ladder bounded on both axes so a bench cannot become a hang.
///
/// `min_hits` is a FRACTION of the swept column, not a constant -- see
/// [`SUPPORT_PERMILLE`].
fn ladder(swept: u64) -> Ladder {
    let min_hits = swept.saturating_mul(SUPPORT_PERMILLE) / 1_000;
    Ladder::with_min_hits(min_hits.max(1)).with_ceiling(50_000)
}

/// One row: two per-unit costs of the same operation, and their quotient.
///
/// Prints whichever way the quotient runs, so a side that got *cheaper* is
/// visible rather than silently passing — a measurement that only ever reports
/// "ok" teaches a reader nothing about which direction it moved.
fn ratio(label: &str, base_ps: u128, at_ps: u128) -> bool {
    if base_ps == 0 || at_ps == 0 {
        println!("  {label:<62} UNMEASURABLE — a side timed at zero");
        return false;
    }
    let up = at_ps.saturating_mul(1_000) / base_ps;
    let down = base_ps.saturating_mul(1_000) / at_ps;
    let ok = up.max(down) <= CEILING_PERMILLE;
    println!(
        "  {label:<62} {base_ps:>9} -> {at_ps:>9} ps   ratio {}.{:03}x  {} {}",
        up / 1_000,
        up % 1_000,
        if ok { "ok" } else { "BREACH" },
        if down > up { "(cheaper)" } else { "" },
    );
    ok
}

/// Picosecond cost of `f`, divided by `units`.
fn per_unit<T>(units: u128, reps: u32, mut f: impl FnMut() -> T) -> u128 {
    // One untimed pass, so a cold cache is not charged to the first column.
    black_box(f());
    let start = Instant::now();
    for _ in 0..reps {
        black_box(f());
    }
    let ps = start.elapsed().as_nanos().saturating_mul(1_000);
    ps / u128::from(reps).max(1) / units.max(1)
}

/// Picosecond cost of `f` per unit, as the minimum over [`TRIALS`] trials.
///
/// [`per_unit`] times one trial and divides. That is enough for a ratio whose
/// two legs are both whole sweeps, and not enough for a floor of one add per
/// bar, which is what C-R-04's floor is. D-0690.
fn per_unit_min<T>(units: u128, reps: u32, mut f: impl FnMut() -> T) -> u128 {
    // One untimed pass, as in `per_unit`.
    black_box(f());
    let mut best = u128::MAX;
    for _ in 0..TRIALS {
        let start = Instant::now();
        for _ in 0..reps {
            black_box(f());
        }
        let ps = start.elapsed().as_nanos().saturating_mul(1_000);
        best = best.min(ps / u128::from(reps).max(1) / units.max(1));
    }
    best
}

/// C-R-01 — the COLUMN BUILD costs the same per bar at every column length.
///
/// # This row measured the wrong thing first, and the measurement said so
///
/// It began as "a RUN costs the same per bar" and read **0.002x** — the short
/// column costing 476 times more per bar than the long one. That is not noise
/// and it is not a regression; it is the claim being false. A run is a column
/// build plus a ladder walk, and the walk is O(candidates), which at a fixed
/// support fraction is combinatorial in the vocabulary and has no business
/// being divided by a bar count at all. `engine`'s own bench header says the
/// join is O(|frontier|²) and states it rather than hiding it.
///
/// So the row now measures the half of a run that IS per-bar. `min_hits` is set
/// to `u64::MAX`, which empties the k=1 frontier, leaving the `Column::build`
/// pass over the bars as nearly all of the work. The other half is not
/// silently dropped — it is [`engine`]'s to bound, and `C-E-01` bounds it.
///
/// **Nearly, and this said "nothing".** Sampled on 2026-09-22 on an arm64
/// laptop, the emptied ladder was about a tenth of this row before D-0677 and a
/// larger share after it, because the build it sits beside got cheaper: the
/// walk still copies the column and tests every k=1 candidate before its
/// frontier empties. The row times what it always timed; only the sentence was
/// wrong. D-0677, D-0680.
///
/// The per-bar floor: the cheapest possible walk over the same bars.
///
/// # Why a ratio alone cannot see a regression
///
/// C-R-01 divides one per-bar cost by another per-bar cost of the same
/// operation, and a UNIFORM slowdown cancels in a quotient. An audit measured
/// exactly that elsewhere in this workspace: a mask operation **174x slower
/// passed its crate's ratio rows at 0.98x-1.00x**, because both legs moved
/// together.
///
/// The denominator has to be something that cannot move when `Column::build`
/// does. This walks the same slice touching each bar's timestamp with one
/// `wrapping_add` — the same loop, the same bounds checks, the same memory
/// traffic, and none of the indicator work. So the quotient is "how many of the
/// cheapest per-bar things does building one bar's condition bits cost".
fn floor_ps_per_bar(bars: &[indicators::Candle]) -> u128 {
    per_unit(u128::try_from(bars.len()).unwrap_or(1), REPS, || {
        let mut acc = 0_i64;
        for b in black_box(bars) {
            acc = acc.wrapping_add(black_box(b).ts_micros);
        }
        black_box(acc)
    })
}

/// The same per-bar floor, timed long enough to hold still.
///
/// The walk is the one [`floor_ps_per_bar`] times: one black-boxed
/// `wrapping_add` per bar over the same slice. Only the timing differs:
/// [`FLOOR_PASSES`] passes per trial instead of five, and the minimum of
/// [`TRIALS`] trials instead of one. C-R-05 uses this floor. C-R-04 does not,
/// because timing its floor this way changes what its 5,000 means, and that
/// recalibration is its own decision (D-0680, D-0690).
fn stable_floor_ps_per_bar(bars: &[indicators::Candle]) -> u128 {
    per_unit_min(
        u128::try_from(bars.len()).unwrap_or(1),
        FLOOR_PASSES,
        || {
            let mut acc = 0_i64;
            for b in black_box(bars) {
                acc = acc.wrapping_add(black_box(b).ts_micros);
            }
            black_box(acc)
        },
    )
}

/// Prints one budget in floors and returns whether it held.
fn budget(label: &str, floor: u128, at_ps: u128, allowed: u128) -> bool {
    if floor == 0 {
        println!("  {label:<58} UNMEASURABLE — the floor timed at zero");
        return false;
    }
    let floors = at_ps.saturating_mul(1_000) / floor;
    let ok = floors <= allowed.saturating_mul(1_000);
    println!(
        "  {label:<58} {at_ps:>8} ps/bar = {}.{:03} floors, budget {allowed}   {}",
        floors / 1_000,
        floors % 1_000,
        if ok { "ok" } else { "OVER BUDGET" }
    );
    ok
}

/// C-R-04 — one bar's column build costs a bounded multiple of the per-bar floor.
fn the_column_build_stays_within_its_budget() -> bool {
    /// Floors allowed per offered bar.
    ///
    /// Measured, arm64 laptop, release, three consecutive runs: **1270.202,
    /// 990.494, 968.863** floors, at a floor of ~336 ps. The 1.31x spread is the
    /// widest of the workspace's budgets, and that is expected: the numerator
    /// runs the whole `Evaluator` — ten indicator families, session rollovers,
    /// an EMA state machine — against a denominator of one add.
    ///
    /// The MAGNITUDE is the point rather than a worry. A thousand of the
    /// cheapest per-bar operations is what turning one bar into 280 condition
    /// bits costs, and knowing that number is the only way to notice it becoming
    /// two thousand.
    ///
    /// **5,000**, sized on the worst observed with roughly 4x left over — the
    /// same rule the other eight budgets apply. It still refuses the 174x
    /// uniform regression: such a build would read about 221,000 floors and be
    /// refused by a factor of 44.
    ///
    /// A breach is NOT a column-length dependence; C-R-01 is that row. It means
    /// every bar got dearer at once, which a quotient cannot see.
    ///
    /// # It breached, and did exactly that job
    ///
    /// First read on CI's x86 runners on 2026-09-20: **5,284 floors, over.**
    /// Every earlier run had failed before this job, so that was also this
    /// budget's first reading off the laptop it was sized on. The overrun was a
    /// real regression rather than the machine: on the same arm64 laptop the
    /// same fixture read 2,436 floors, 1.9 to 2.5 times the three readings
    /// above. D-0677 reduced it with two exact fixes — a 128-bit division per
    /// Fibonacci rung, and one known-mask table rebuilt on every bar — to 703,513
    /// ps per bar on that laptop, −26.7%, still 1.81 times the 388,075 read at
    /// `a8814e64`. It did NOT touch this number.
    ///
    /// CI then read 2,967.491 floors (run 35767619936), but over a floor of 643
    /// ps against 309 ps on 20 September. This floor is timed over about 10 µs,
    /// so across runners a reading near the budget cannot tell the code from the
    /// machine. D-0680.
    const ALLOWED: u128 = 5_000;

    let bars = synthetic::sessions(8);
    let floor = floor_ps_per_bar(&bars);
    println!("  the per-bar floor is {floor} ps — one black-boxed wrapping_add");
    let neutered = Ladder::with_min_hits(u64::MAX);
    let at = per_unit(u128::try_from(bars.len()).unwrap_or(1), REPS, || {
        Sweeper::new(neutered).run(black_box(&bars), &mut evaluator())
    });
    budget(
        "C-R-04 column build against the per-bar floor",
        floor,
        at,
        ALLOWED,
    )
}

/// C-R-05 — the column build a CASH EQUITY runs costs a bounded multiple of
/// the stable per-bar floor.
///
/// # Why C-R-04 does not already cover it
///
/// Every other row in this file, and every row in `crates/indicators`' bench,
/// builds with `Availability::Absent`. `cli`'s `stored::vwap_availability`
/// binds `Present` for `Kind::Equity`, which turns the VWAP family on: three
/// `i128` accumulators per bar, a square root for the bands, and the VWAP
/// positions' known mask. A slowdown confined to that family passed every
/// Gate 8 row. D-0690.
///
/// The fixture is C-R-04's, `synthetic::sessions(8)`, whose bars carry volume
/// 1,000 to 1,010. Two setup checks run before anything is timed, because a
/// row that silently timed the index path under an equity label would pass
/// for the wrong reason: the fixture must carry volume, and the `Present`
/// column must refuse no bar and differ from the `Absent` one.
///
/// The same build with VWAP off is printed beside it against the SAME floor,
/// as context, not judged. It is C-R-04's numerator timed this row's way, and
/// the calibration figure D-0680 said C-R-04 lacks.
fn the_equity_column_build_stays_within_its_budget() -> bool {
    /// Floors allowed per offered bar.
    ///
    /// Measured 2026-09-23 on an Apple M4 Pro laptop (Mac16,7, 10
    /// performance and 4 efficiency cores), release bench profile, eight
    /// runs, while other worktrees built and tested on the same machine. Runs
    /// 1 to 5 sized the budget; runs 6 to 8 were verification runs with the
    /// budget in place, at a much higher load:
    ///
    /// | run | load | stable floor | VWAP on, ps/bar | floors   | VWAP off, ps/bar | floors   |
    /// |-----|------|--------------|-----------------|----------|------------------|----------|
    /// | 1   | 11   | 336 ps       | 1,022,969       | 3044.550 | 710,334          | 2114.089 |
    /// | 2   | 12   | 318 ps       | 1,002,681       | 3153.084 | 705,709          | 2219.210 |
    /// | 3   | 12   | 325 ps       | 1,013,613       | 3118.809 | 702,776          | 2162.387 |
    /// | 4   | 11   | 324 ps       | 1,011,191       | 3120.959 | 708,708          | 2187.370 |
    /// | 5   | 11   | 321 ps       | 1,026,701       | 3198.445 | 720,666          | 2245.065 |
    /// | 6   | 43   | 366 ps       | 1,178,744       | 3220.612 | 843,469          | 2304.560 |
    /// | 7   | 42   | 383 ps       | 1,157,901       | 3023.240 | 862,020          | 2250.704 |
    /// | 8   | 59   | 405 ps       | 1,472,309       | 3635.330 | 815,931          | 2014.644 |
    ///
    /// "load" is the one-minute load average on 14 cores. The stable floor
    /// spread 1.057x over runs 1 to 5 and 1.274x over all eight. C-R-04's
    /// floor, in the same eight runs, read 463, 452, 455, 513, 458, 1,275, 541
    /// and 1,252 ps, a 2.82x spread, and 555 ps in a run before this row
    /// existed.
    ///
    /// **14,500**, sized on the worst observed, 3,635.330 in run 8, with
    /// roughly 4x left over (3.99x), the rule C-R-04's 5,000 was sized by. A
    /// 174x uniform regression would read about 632,500 floors and be refused
    /// by a factor of 43.
    ///
    /// **Not measured on CI.** Take the worst of both CI figures D-0680 records
    /// for C-R-04: an index build of 1,908,097 ps per bar, 2.76 times the
    /// 691,505 C-R-04 read here, and a floor as low as 309 ps. Scaled that way,
    /// run 5's numerator, the worst of runs 1 to 5, would read about 9,170
    /// floors, and run 8's about 13,150. Both are inside the budget, both are
    /// extrapolations rather than readings, and the first CI run is the
    /// calibration check.
    const ALLOWED: u128 = 14_500;

    let bars = synthetic::sessions(8);
    if Vwap::availability_of(&bars) != Availability::Present {
        println!("  C-R-05 UNMEASURABLE — the fixture carries no volume, so VWAP cannot run");
        return false;
    }
    let equity = Column::build(&bars, &mut evaluator_with(Availability::Present));
    let index = Column::build(&bars, &mut evaluator());
    if equity.census().refused() != 0 || equity.bits() == index.bits() {
        println!(
            "  C-R-05 UNMEASURABLE — the Present build refused a bar or computed \
             nothing the Absent build did not"
        );
        return false;
    }

    let units = u128::try_from(bars.len()).unwrap_or(1);
    let floor = stable_floor_ps_per_bar(&bars);
    println!(
        "  the stable per-bar floor is {floor} ps — {FLOOR_PASSES} passes, minimum of {TRIALS} trials"
    );
    let neutered = Ladder::with_min_hits(u64::MAX);
    let at = per_unit_min(units, BUILD_REPS, || {
        Sweeper::new(neutered).run(black_box(&bars), &mut evaluator_with(Availability::Present))
    });
    let off = per_unit_min(units, BUILD_REPS, || {
        Sweeper::new(neutered).run(black_box(&bars), &mut evaluator())
    });
    let off_floors = off.saturating_mul(1_000).checked_div(floor).unwrap_or(0);
    println!(
        "  {:<58} {off:>8} ps/bar = {}.{:03} floors (context, NOT judged)",
        "C-R-05 context: index build (VWAP off), stable floor",
        off_floors / 1_000,
        off_floors % 1_000,
    );
    budget(
        "C-R-05 equity column build (VWAP on), stable floor",
        floor,
        at,
        ALLOWED,
    )
}

fn the_column_build_costs_the_same_per_bar_at_every_column_length() -> bool {
    let short = synthetic::sessions(8);
    let long = synthetic::sessions(32);
    // Divided by OFFERED bars, and the measurement is what settled which.
    //
    // Dividing by SWEPT bars read 0.348x -- the short column apparently costing
    // 2.87x more per bar. It is not a cost difference. `Column::build` steps
    // EVERY offered bar; the warm-up prefix is a fixed 1,876 of them, so a
    // swept-bar divisor charges that constant to 1,124 bars in one column and to
    // 10,124 in the other. Predicted quotient 0.444 against a measured 0.348,
    // which is the artifact and not a regression.
    //
    // The build is per offered bar because it touches every offered bar. The
    // swept count is the right divisor for the LADDER, which is not what this
    // row measures.
    let neutered = Ladder::with_min_hits(u64::MAX);
    let base = per_unit(u128::try_from(short.len()).unwrap_or(1), REPS, || {
        Sweeper::new(neutered).run(black_box(&short), &mut evaluator())
    });
    let at = per_unit(u128::try_from(long.len()).unwrap_or(1), REPS, || {
        Sweeper::new(neutered).run(black_box(&long), &mut evaluator())
    });
    ratio(
        "C-R-01 column build, ps/offered bar: 3,000 -> 12,000 bars",
        base,
        at,
    )
}

/// C-R-02 — the audit costs nothing per bar.
///
/// # Holding the levels equal is the whole design of this row
///
/// Render cost is `fixed prologue + per level`. Dividing by levels therefore
/// falls as the ladder deepens -- the first version of this row read 0.197x for
/// exactly that reason, which says nothing about bars. `report.rs` claims it
/// "costs one string per run rather than anything per bar", so the fixture
/// neuters BOTH ladders to one empty level and varies only the column: four
/// times the bars, the same levels, and the quotient is then about bars alone.
fn the_report_costs_nothing_per_bar() -> bool {
    let short = synthetic::sessions(8);
    let long = synthetic::sessions(32);
    let neutered = Ladder::with_min_hits(u64::MAX);
    let a = Sweeper::new(neutered).run(&short, &mut evaluator());
    let b = Sweeper::new(neutered).run(&long, &mut evaluator());
    assert_eq!(
        a.sweep.levels.len(),
        b.sweep.levels.len(),
        "the fixture must hold levels equal or this row measures depth, not bars"
    );
    // The label below states two bar counts. Asserting them keeps the printed row
    // from quietly becoming a caption for a fixture that changed underneath it.
    assert_eq!((a.census.swept, b.census.swept), (1_124, 10_124));
    // Per LEVEL on purpose. Dividing by bars would make a render whose cost is
    // flat in the column look like it got four times cheaper, and the row would
    // pass for the wrong reason.
    let base = per_unit(
        u128::try_from(a.sweep.levels.len()).unwrap_or(1),
        REPS_FAST,
        || report::render(black_box(&a), None),
    );
    let at = per_unit(
        u128::try_from(b.sweep.levels.len()).unwrap_or(1),
        REPS_FAST,
        || report::render(black_box(&b), None),
    );
    ratio(
        "C-R-02 report, ps/level: 1,124 bars -> 10,124 bars",
        base,
        at,
    )
}

/// C-R-03 — the identity does not depend on how many bars the digest covered.
fn the_identity_does_not_depend_on_how_many_bars_the_digest_covered() -> bool {
    let short = synthetic::sessions(8);
    let long = synthetic::sessions(32);
    let Ok(key) = InstrumentKey::index(Exchange::Nse, "NIFTY") else {
        eprintln!("NIFTY is a swept instrument; this cannot fail");
        std::process::exit(2);
    };
    let run = |digest| Run {
        mask: ConditionMask::default(),
        direction: Direction::Undirected,
        instrument: &key,
        timeframe: "1min",
        params: Params::of(ladder(1)),
        data_digest: digest,
        commit: "0123456789abcdef",
        // A fixed word, like `commit` and `timeframe` above: this row measures
        // that `identity` is O(1) in the BAR COUNT, and every term but
        // `data_digest` is held constant so the digest is the only thing varying
        // across the two magnitudes.
        feed: "groww",
    };
    // The digests are computed OUTSIDE the timed region. `data_digest` is
    // O(bars) and honestly so -- it must read every bar. What this row claims is
    // that `identity` cannot see that, because it receives thirty-two bytes.
    let d_short = data_digest(&short);
    let d_long = data_digest(&long);
    let base = per_unit(1, REPS_FAST, || identity(black_box(&run(d_short))));
    let at = per_unit(1, REPS_FAST, || identity(black_box(&run(d_long))));
    ratio(
        "C-R-03 identity, ps/call: digest of 1,124 -> 10,124 bars",
        base,
        at,
    )
}

fn main() {
    println!("crates/runner — ratio bench, ceiling {CEILING_PERMILLE} permille");
    let rows = [
        the_column_build_costs_the_same_per_bar_at_every_column_length(),
        the_report_costs_nothing_per_bar(),
        the_identity_does_not_depend_on_how_many_bars_the_digest_covered(),
        the_column_build_stays_within_its_budget(),
        the_equity_column_build_stays_within_its_budget(),
    ];
    let breached = rows.iter().filter(|ok| !**ok).count();
    if breached > 0 {
        println!("\n{breached} row(s) BREACHED. A per-unit cost that grows with the");
        println!("input is the defect gate 8 exists to catch. CLAUDE.md section 3 rule 4.");
        std::process::exit(1);
    }
    println!("\nok — every per-unit cost is flat within the ceiling.");
}
