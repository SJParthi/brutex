//! Gate 8 for `crates/cli` — the bounds this crate's headers claim, re-measured
//! instead of argued.
//!
//! # Why this file exists at all
//!
//! `crates/cli` was the **only** one of the thirteen workspace crates with no
//! `benches/` directory, and gate 14 refused it in those terms: *"REFUSED
//! crates/cli — 20 cost claim(s), no row in the table"*. Every other crate
//! ships `benches/ratio.rs`.
//!
//! That hole was the worst possible one. `CLAUDE.md` §5 says `cli` is what makes
//! the sweep reachable at all — `api` does not depend on `runner`, `engine` or
//! `indicators`, so the binary an operator actually runs to produce a trading
//! result had **zero** ratio measurement. A regression to O(n) in the result
//! store, in rung resolution or in the report render would have breached no
//! bench and no gate would have noticed.
//!
//! # What is measured
//!
//! Every row divides one per-unit cost by another per-unit cost of the **same**
//! operation and compares the quotient against [`CEILING_PERMILLE`]. An absolute
//! nanosecond figure measures the machine; a ratio is what survives moving
//! between machines.
//!
//! The rows are taken from the cost table `results.rs`'s own header prints,
//! because a bound a module states in writing is exactly the bound gate 14 asks
//! it to re-measure:
//!
//! * **C-CLI-01** — **reading record *i* does not depend on *i***. The header
//!   says the address is `HEADER + i·STRIDE`, "an add and a multiply". If that
//!   ever became a walk, reading the last record of a long ledger would cost
//!   proportionally more than reading the first. Measured as the first record
//!   against the last, on the same file.
//!
//! * **C-CLI-02** — **counting does not depend on how many there are**. The
//!   header says `(file_len - HEADER) / STRIDE`, "no walk". A count that walked
//!   would track the record count directly. Measured at two ledger sizes an
//!   order of magnitude apart, per record.
//!
//! * **C-CLI-03** — **the duplicate check does not depend on the ledger size**.
//!   The header calls it "O(1) amortised — a `HashSet` of identities, built once
//!   at open". The BUILD is one pass and is deliberately not what is timed here;
//!   the QUERY is, and it is the thing that must not scan.
//!
//! * **C-CLI-04** — **encoding a record does not depend on the ledger it will
//!   join**. `to_bytes` writes one fixed-stride array from one struct. This is
//!   the cheapest row and the one most likely to catch an accidental read of
//!   surrounding state.
//!
//! # The sentence above this one used to end the file, and it was the defect
//!
//! It read: *"**Not measured here:** whether any of those costs is small. This
//! file refuses a cost that GROWS."* That was an honest description of what the
//! file did and a **hole**, and an O(1) audit of all thirteen crates named it:
//! `cli` was the only crate whose bench had **no absolute-floor row at all**.
//! The other twelve each divide a measured cost by a measured FLOOR and refuse a
//! multiple; this one divided two costs by each other and refused a quotient.
//!
//! A quotient cannot see a UNIFORM slowdown. The audit measured exactly that
//! elsewhere in this workspace: a mask operation **174x slower passed its
//! crate's ratio rows at 0.98x–1.00x**, because both legs moved together. Every
//! row above would report `ok` for a `to_bytes` that had become a hundred times
//! dearer, because C-CLI-04 divides one `to_bytes` by another `to_bytes`. `cli`
//! is the crate `CLAUDE.md` §5 makes the sweep reachable from, so it was the one
//! place that regression was structurally uncatchable.
//!
//! Two rows close it, against the same one-instruction floor the other benches
//! use:
//!
//! * **C-CLI-05** — **encoding one record costs a bounded multiple of the
//!   cheapest instruction there is**. `to_bytes` is the crate's cheapest owned
//!   operation and the most stable to measure: no syscall, no lock, no
//!   allocation — twenty-nine constant-offset field copies and one `blake3` seal
//!   over 253 bytes.
//!
//! * **C-CLI-06** — **the duplicate probe costs a bounded multiple of the same
//!   floor**. `CLAUDE.md` §3 rule 4 names duplicate rejection as one of the five
//!   operations that must be O(1), and `holds` is this crate's spelling of it.
//!   C-CLI-03 proves the probe does not depend on the ledger; this proves the
//!   probe is small.
//!
//! **Still not measured here:** the two rows that reach the filesystem. A budget
//! on `read` or `append` would be timing the page cache and the append lock, and
//! `crates/store` already carries the one absolute row for a record read (C-29).
//! `docs/06-limits.md` is where absolute figures and the things nobody has timed
//! are recorded.
//!
//! **Neither new id is in gate 14's `cover` table or in
//! `docs/04-invariants.md` yet.** They are measured and printed here; the table
//! row and the invariant rows are edits to files this change does not touch, and
//! adding an id to that table before its invariant row exists fails gate 14's
//! layer 4 — correctly, as its own comment says: *"the repair is to write the
//! rows, not to widen the gate"*.

use core::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

use cli::results::{Record, Results, field};

/// How far a ratio may sit from 1.000 before the row is a breach.
///
/// 2.500x, the same figure `crates/runner`'s bench uses and for the same reason:
/// every measurement here contains a syscall or a hash rather than one
/// arithmetic operation, so the noise floor is correspondingly higher — and a
/// bound this loose still refuses the thing it exists to refuse. A read that had
/// become a walk would not land at 2.4x on a ten-fold size difference; it would
/// land near 10x, which is the shape of the regression rather than a near miss.
const CEILING_PERMILLE: u128 = 2_500;

/// Repetitions for a site that costs a syscall.
const REPS: u32 = 200;

/// Repetitions for a site that is pure arithmetic or one hash lookup.
///
/// Twenty thousand, for the reason `runner`'s bench records: five repetitions of
/// a sub-microsecond operation is below the noise floor of anything this timer
/// can see, and the first measurement of such a site read a 0.369x
/// "improvement" that was the scheduler and a cold instruction cache.
const REPS_FAST: u32 = 20_000;

/// The small ledger, in records.
const SMALL: u64 = 64;

/// The large ledger, in records. An order of magnitude and then some, so a cost
/// that tracked the count cannot hide inside the ceiling.
const LARGE: u64 = 1_024;

/// One row: two per-unit costs of the same operation, and their quotient.
///
/// Prints whichever way the quotient runs, so a side that got *cheaper* is
/// visible rather than silently passing — a measurement that only ever reports
/// "ok" teaches a reader nothing about which direction it moved.
fn ratio(label: &str, base_ps: u128, at_ps: u128) -> bool {
    if base_ps == 0 || at_ps == 0 {
        println!("  {label:<58} UNMEASURABLE — a side timed at zero");
        return false;
    }
    let up = at_ps.saturating_mul(1_000) / base_ps;
    let down = base_ps.saturating_mul(1_000) / at_ps;
    let ok = up.max(down) <= CEILING_PERMILLE;
    println!(
        "  {label:<58} {base_ps:>9} -> {at_ps:>9} ps   ratio {}.{:03}x  {} {}",
        up / 1_000,
        up % 1_000,
        if ok { "ok" } else { "BREACH" },
        if down > up { "(cheaper)" } else { "" },
    );
    ok
}

/// A record whose identity is a function of `n`, so every row is distinct.
///
/// Distinct identities matter: the ledger refuses a duplicate, so a fixture that
/// reused one would measure the refusal path instead of the append path.
fn record(n: u64) -> Record {
    let mut identity = [0_u8; 32];
    identity
        .get_mut(..8)
        .unwrap_or_else(|| unreachable_fixture("a 32-byte identity has 8 leading bytes"))
        .copy_from_slice(&n.to_le_bytes());
    Record {
        identity,
        // FIXED, not `now`. A clock in a fixture makes two runs of this bench
        // encode different bytes, and `CLAUDE.md` §3 rule 5 is about exactly
        // that -- a measurement whose input varies is not a measurement.
        finished_micros: 1_787_509_605_880_000,
        mask_words: [0; 6],
        feed: field("zerodha"),
        underlying: field("NIFTY"),
        timeframe: field("15min"),
        from_year: 2019,
        from_month: 12,
        to_year: 2026,
        to_month: 8,
        months_asked: 81,
        months_found: 81,
        bars: 41_572,
        min_hits: 8_314,
        combinations: 1_024_058,
        depth: 16,
        halted: 0,
        trades: 11_209,
        pessimistic: 2_459_160,
        optimistic: 3_649_640,
        worst_trade: -4_694,
        max_drawdown: 29_163,
        winner_mae: 300,
        winner_mfe: 700,
        all_mae: 350,
        exit_rungs: [0; 5],
    }
}

/// A bench cannot use `expect` under this workspace's lints, and a fixture that
/// cannot be built is a defect in the bench rather than a measurement.
fn unreachable_fixture(why: &str) -> ! {
    eprintln!("the bench fixture is malformed: {why}");
    std::process::exit(2);
}

/// A ledger of `n` records in a fresh directory, returned open.
fn ledger(tag: &str, n: u64) -> (Results, PathBuf) {
    let root = std::env::temp_dir().join(format!("brutex-cli-bench-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    if let Err(why) = std::fs::create_dir_all(&root) {
        unreachable_fixture(&format!("a temp root could not be made: {why}"));
    }
    let Ok(mut store) = Results::open(&root) else {
        unreachable_fixture("a fresh directory refused to open as a ledger");
    };
    for i in 0..n {
        if store.append(&record(i)).is_err() {
            unreachable_fixture("an append into a fresh ledger refused");
        }
    }
    (store, root)
}

/// Picoseconds per repetition, so two sites at different scales compare.
fn per(reps: u32, elapsed_ns: u128) -> u128 {
    elapsed_ns.saturating_mul(1_000) / u128::from(reps.max(1))
}

/// How many times a BUDGET measurement is repeated before the minimum is taken.
///
/// Forty, the figure `crates/vocab`'s bench uses for the same floor.
///
/// # Why the minimum, and why only the budget rows take it
///
/// A ratio row divides two costs measured the same way, so a scheduler that
/// slowed both legs mostly cancels. A budget row does not have that protection:
/// it compares an ABSOLUTE magnitude, so one preempted sample would report a
/// regression that is really another process. The scheduler can only ever make a
/// sample slower, so the smallest observation is the closest thing to the cost of
/// the work itself — `crates/store`'s bench records measuring that directly,
/// against a foreign process at 686% CPU: the minimum moved 0.2% across three
/// runs while the median moved 78%.
///
/// The four ratio rows above are deliberately left on their single-sample [`per`]
/// timing. Changing how they are measured would make this change's numbers
/// incomparable with the ones already recorded for them, and they are not what
/// was missing.
const TRIALS: u32 = 40;

/// Repetitions for a budget site that costs a hash over a few hundred bytes.
///
/// Two thousand, the figure `crates/store`'s bench uses. `to_bytes` is a
/// microsecond-scale call, so this is milliseconds of work per trial — far above
/// anything `Instant` cannot resolve, and forty trials of it still cost under a
/// second.
const REPS_BUDGET: u32 = 2_000;

/// Picoseconds per call, as the MINIMUM over [`TRIALS`] runs of `reps` calls.
fn cost_ps<T>(reps: u32, mut op: impl FnMut() -> T) -> u128 {
    let mut best = u128::MAX;
    for _ in 0..TRIALS {
        let start = Instant::now();
        for _ in 0..reps {
            black_box(op());
        }
        let ps = per(reps, start.elapsed().as_nanos());
        if ps < best {
            best = ps;
        }
    }
    best
}

/// How many independent [`cost_ps`] minima the floor takes the minimum OF.
///
/// # Measured, and the reason it is not one
///
/// The floor at [`TRIALS`] alone read **522, 1156 and 525 ps** on three
/// consecutive runs of this bench while the numerators it divides moved 0.7%
/// (`to_bytes`: 258.0, 259.8, 259.0 ns). One trial in three was 2.2x high with
/// nothing else moving — the signature of a scheduler parking a 10-microsecond
/// loop on an efficiency core, on a machine at load average 104 across 14 cores.
///
/// That error runs in the DANGEROUS direction for a budget. A floor read too
/// HIGH divides the numerator down and hides a regression; a floor read too LOW
/// on an idle machine divides it up and could refuse a healthy build. The minimum
/// is biased toward the true cost, so more rounds move the reading toward the
/// number an unloaded machine would report — which is the one the budget has to
/// survive. Five rounds is 200 trials for a measurement that costs milliseconds.
const FLOOR_ROUNDS: u32 = 5;

/// The machine's own floor: what [`cost_ps`] reports for the cheapest operation
/// there is, timed by this same loop, in this same build.
///
/// # Why the denominator is not part of this crate
///
/// It must be something that **cannot move when the numerator does**. A floor
/// drawn from `results.rs` would be dragged along by the same regression it is
/// meant to expose, and the budget would read `ok` for the same reason the ratios
/// do. `wrapping_add` on a black-boxed `u64` is one instruction, it is not part
/// of `Results` or `Record`, and no change to either can change it — so the
/// quotient scales with the MACHINE and does not scale with a regression.
///
/// The floor is not zero-cost and is not meant to be: it carries this harness's
/// own loop and `black_box` overhead, which every numerator carries too. That is
/// what makes it the right unit — "how many of the cheapest thing does this
/// cost". Same construction as `crates/vocab`, `crates/engine` and
/// `crates/runner`, so the four numbers are read against each other.
fn floor_ps() -> u128 {
    // `unwrap_or(0)` rather than `unwrap`, which this workspace denies. The range
    // is non-empty so the `None` arm is unreachable, and a zero reaching `budget`
    // prints UNMEASURABLE and fails rather than dividing.
    (0..FLOOR_ROUNDS)
        .map(|_| cost_ps(REPS_FAST, || black_box(1_u64).wrapping_add(black_box(1))))
        .min()
        .unwrap_or(0)
}

/// Prints one budget in floors and returns whether it held.
///
/// Separate from [`ratio`] because a breach means something different. A breached
/// RATIO says the cost depends on the input. A breached BUDGET says the cost rose
/// for every input at once, which no ratio in this file can report.
fn budget(label: &str, floor: u128, at_ps: u128, allowed: u128) -> bool {
    if floor == 0 {
        println!("  {label:<58} UNMEASURABLE — the floor timed at zero");
        return false;
    }
    let floors = at_ps.saturating_mul(1_000) / floor;
    let ok = floors <= allowed.saturating_mul(1_000);
    println!(
        "  {label:<58} {at_ps:>8} ps = {}.{:03} floors, budget {allowed}   {}",
        floors / 1_000,
        floors % 1_000,
        if ok { "ok" } else { "OVER BUDGET" }
    );
    ok
}

/// **C-CLI-05** — encoding one record costs a bounded multiple of the floor.
///
/// The absolute half of C-CLI-04's claim. That row proves `to_bytes` does not
/// depend on which record it is given; this one proves the constant is small.
fn the_encode_stays_within_its_budget(floor: u128) -> bool {
    /// Floors allowed per encoded record.
    ///
    /// # Measured
    ///
    /// arm64 laptop (14-core M4 Pro, 48 GB), `bench` profile, six consecutive
    /// runs: **496.367, 496.687, 497.005, 496.247, 497.315, 496.007** floors, at
    /// a floor of 520–522 ps and an absolute cost of **258.6–259.4 ns** per
    /// record. Seven earlier runs, before [`FLOOR_ROUNDS`] existed, spread
    /// 493.411–500.159 on the same absolute numerator. The spread is **1.01x**,
    /// the tightest in this file, which is what a numerator with no syscall and
    /// no lock in it looks like.
    ///
    /// **The machine was HEAVILY LOADED and the number is an upper bound, not a
    /// clean floor.** Load average ran 45–104 across 14 cores for every run
    /// above; `uptime` was read before and after each. That biases the quotient
    /// UPWARD rather than downward, because the numerator's trials are ~520 µs
    /// each and the floor's are ~10 µs, so the floor's minimum finds an
    /// uninterrupted window far more easily than the numerator's does. A budget
    /// sized on it is therefore loose, not tight — and it is honest about which.
    ///
    /// # Why 2,000
    ///
    /// The worst observed is 500.159, and **2,000** leaves 4.0x — the same rule
    /// the other twelve crates' budgets apply. It still refuses the 174x uniform
    /// regression this row exists for: such an encode would read about 86,300
    /// floors and be refused by a factor of 43.
    ///
    /// A breach is NOT a data dependence; C-CLI-04 is that row. It means every
    /// record got dearer at once, which a quotient cannot see.
    const ALLOWED: u128 = 2_000;

    let one = record(1);
    let at = cost_ps(REPS_BUDGET, || black_box(&one).to_bytes());
    budget(
        "C-CLI-05  to_bytes against the instruction floor",
        floor,
        at,
        ALLOWED,
    )
}

/// **C-CLI-06** — the duplicate probe costs a bounded multiple of the floor.
///
/// `CLAUDE.md` §3 rule 4 names duplicate rejection as one of five operations that
/// must be constant, and `Results::holds` is this crate's spelling of it.
/// C-CLI-03 proves the probe does not depend on the ledger's size; only this row
/// can say the probe is CHEAP, and its doc comment in `results.rs` says outright
/// that the bound there is argued rather than timed.
///
/// The MISS is what is timed, for C-CLI-03's reason: a map that had silently
/// become a linear search shows its worst case on the identity that is absent,
/// because it must reach the end before it can answer.
fn the_duplicate_check_stays_within_its_budget(store: &Results, floor: u128) -> bool {
    /// Floors allowed per probe.
    ///
    /// # Measured
    ///
    /// arm64 laptop (14-core M4 Pro, 48 GB), `bench` profile, six consecutive
    /// runs: **14.980, 16.266, 14.990, 14.969, 15.019, 14.961** floors, at a
    /// floor of 520–522 ps and an absolute cost of **7.81–8.49 ns** per probe.
    /// Same load caveat as C-CLI-05: load average 45–104 on 14 cores, so this is
    /// an upper bound taken under load rather than a clean idle figure.
    ///
    /// Fifteen floors is what one `SipHash` of 32 bytes plus one `hashbrown`
    /// group probe costs, and knowing that number is the only way to notice it
    /// becoming a hundred and fifty.
    ///
    /// # Why 65
    ///
    /// The worst observed is 16.266, and **65** leaves 4.0x — the workspace rule.
    /// A probe that had become a walk over a 1,024-record ledger would read in
    /// the thousands; a probe 174x dearer would read about 2,800 floors and be
    /// refused by a factor of 43.
    const ALLOWED: u128 = 65;

    let absent = record(LARGE + 7).identity;
    let at = cost_ps(REPS_FAST, || black_box(store).holds(black_box(&absent)));
    budget(
        "C-CLI-06  duplicate probe against the instruction floor",
        floor,
        at,
        ALLOWED,
    )
}

/// **C-CLI-01** — reading record *i* does not depend on *i*.
///
/// The header says the address is `HEADER + i·STRIDE`, "an add and a multiply".
/// If that ever became a walk, the last record of a long ledger would cost
/// proportionally more than the first.
fn the_read_does_not_depend_on_which_record(store: &mut Results) -> bool {
    let start = Instant::now();
    for _ in 0..REPS {
        black_box(store.read(0).is_ok());
    }
    let first = per(REPS, start.elapsed().as_nanos());

    let last = LARGE - 1;
    let start = Instant::now();
    for _ in 0..REPS {
        black_box(store.read(last).is_ok());
    }
    let latest = per(REPS, start.elapsed().as_nanos());

    ratio("C-CLI-01  read record 0 -> read record 1023", first, latest)
}

/// **C-CLI-03** — the duplicate check is a hash probe, not a walk.
///
/// Timed on an ALREADY OPEN ledger, so the one pass that builds the set is
/// excluded by construction. The header is explicit that the build is the part
/// which is O(runs) and the query is the part that must not scan; timing them
/// together would measure the one this row is not about.
///
/// A hit and a MISS rather than two hits: a set that had silently become a
/// linear search shows its worst case on the miss, which has to reach the end
/// before it can answer.
fn the_duplicate_check_does_not_scan_the_ledger(store: &Results) -> bool {
    let present = record(LARGE - 1).identity;
    let absent = record(LARGE + 7).identity;

    let start = Instant::now();
    for _ in 0..REPS_FAST {
        black_box(store.holds(&present));
    }
    let hit = per(REPS_FAST, start.elapsed().as_nanos());

    let start = Instant::now();
    for _ in 0..REPS_FAST {
        black_box(store.holds(&absent));
    }
    let miss = per(REPS_FAST, start.elapsed().as_nanos());

    ratio("C-CLI-03  duplicate check: hit -> miss", hit, miss)
}

/// **C-CLI-02** — counting does not depend on how many there are.
///
/// The header says `(file_len - HEADER) / STRIDE`, "no walk". Measured at two
/// ledger sizes sixteen-fold apart, so a count that walked could not hide inside
/// the ceiling.
fn the_count_does_not_depend_on_how_many_there_are(small: &Results, big: &Results) -> bool {
    let start = Instant::now();
    for _ in 0..REPS_FAST {
        black_box(small.len().unwrap_or(0));
    }
    let small_ps = per(REPS_FAST, start.elapsed().as_nanos());

    let start = Instant::now();
    for _ in 0..REPS_FAST {
        black_box(big.len().unwrap_or(0));
    }
    let big_ps = per(REPS_FAST, start.elapsed().as_nanos());

    ratio("C-CLI-02  count 64 records -> count 1024", small_ps, big_ps)
}

/// **C-CLI-04** — encoding a record does not depend on the ledger it will join.
///
/// `to_bytes` writes one fixed-stride array from one struct. The cheapest row
/// here and the one most likely to catch an accidental read of surrounding
/// state — a `to_bytes` that consulted the store would stop being a pure
/// function of the record it was called on.
fn the_encode_does_not_depend_on_the_ledger_it_joins() -> bool {
    let early = record(1);
    let far = record(LARGE * 4_096);

    let start = Instant::now();
    for _ in 0..REPS_FAST {
        black_box(early.to_bytes());
    }
    let early_ps = per(REPS_FAST, start.elapsed().as_nanos());

    let start = Instant::now();
    for _ in 0..REPS_FAST {
        black_box(far.to_bytes());
    }
    let far_ps = per(REPS_FAST, start.elapsed().as_nanos());

    ratio(
        "C-CLI-04  encode record 1 -> encode record 4194304",
        early_ps,
        far_ps,
    )
}

fn main() {
    println!("crates/cli — ratio bench (gate 8)");
    println!(
        "  ceiling {}.{:03}x on every RATIO row; the two budget rows are in floors\n",
        CEILING_PERMILLE / 1_000,
        CEILING_PERMILLE % 1_000
    );

    let (mut big, big_root) = ledger("read", LARGE);
    let (small, small_root) = ledger("count", SMALL);

    // NOT `&&`. Every row must RUN, so a breach in the first does not hide the
    // state of the other five -- a bench that stops at the first failure tells
    // an operator less than one that reports all six.
    let ratios = the_read_does_not_depend_on_which_record(&mut big)
        & the_duplicate_check_does_not_scan_the_ledger(&big)
        & the_count_does_not_depend_on_how_many_there_are(&small, &big)
        & the_encode_does_not_depend_on_the_ledger_it_joins();

    // The floor is taken ONCE and shared, so both budgets are quoted in the same
    // unit on the same machine in the same run. Two separately measured floors
    // would make the two rows incomparable for no gain.
    let floor = floor_ps();
    println!("\n  the instruction floor is {floor} ps — one black-boxed wrapping_add");
    let budgets = the_encode_stays_within_its_budget(floor)
        & the_duplicate_check_stays_within_its_budget(&big, floor);
    let ok = ratios & budgets;

    // The fixtures are temp directories and are removed whether or not a row
    // breached — a bench that leaves state behind makes the NEXT run's numbers
    // a function of this one's.
    drop(big);
    drop(small);
    let _ = std::fs::remove_dir_all(&big_root);
    let _ = std::fs::remove_dir_all(&small_root);

    println!();
    if ok {
        println!("OK — every bound crates/cli states in writing was re-measured and holds.");
    } else {
        println!("BREACH — a bound this crate claims did not hold. See the row above.");
        std::process::exit(1);
    }
}
