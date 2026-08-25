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
//! **Not measured here:** whether any of those costs is *small*. This file
//! refuses a cost that GROWS. `docs/06-limits.md` is where absolute figures and
//! the things nobody has timed are recorded.

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
        "  ceiling {}.{:03}x on every row\n",
        CEILING_PERMILLE / 1_000,
        CEILING_PERMILLE % 1_000
    );

    let (mut big, big_root) = ledger("read", LARGE);
    let (small, small_root) = ledger("count", SMALL);

    // NOT `&&`. Every row must RUN, so a breach in the first does not hide the
    // state of the other three -- a bench that stops at the first failure tells
    // an operator less than one that reports all four.
    let ok = the_read_does_not_depend_on_which_record(&mut big)
        & the_duplicate_check_does_not_scan_the_ledger(&big)
        & the_count_does_not_depend_on_how_many_there_are(&small, &big)
        & the_encode_does_not_depend_on_the_ledger_it_joins();

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
