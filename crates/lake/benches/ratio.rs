//! Gate 8 for `crates/lake` — reading a row out of a decoded batch costs the
//! same whatever size the batch is.
//!
//! # What is measured, and what is deliberately not
//!
//! `crates/lake` makes exactly one cost claim: [`lake::batch::Batch::row`] is
//! O(1). Everything else in the crate is honestly super-constant and says so —
//! opening a file is O(file bytes) and decoding a row group is O(bytes in the
//! group), because every page in it must be decompressed. Benching those would
//! measure zstd, not this crate.
//!
//! So this file measures the one operation that carries a bound, at three
//! magnitudes: 2,480 rows (the real row count of
//! `NSE-NIFTY-01Apr20-10000-CE/1minute/2020/03.parquet`), 24,800 and 248,000.
//! A `row` that scanned — or that rebuilt a bar by walking a vector of
//! structs — would show the largest batch costing a hundred times the
//! smallest, and the ratio lines below would say BREACH.
//!
//! The last-row measurements are the ones that matter. A lookup at index 0 is
//! cheap under any implementation, including a linear scan; it is the cost at
//! the *end* of a growing batch that separates an index from a walk.
//!
//! See `crates/store/benches/ratio.rs` for why there is no benchmarking
//! framework and why every ratio here is integer arithmetic.

use std::hint::black_box;
use std::time::Instant;

use lake::batch::Batch;
use lake::contract::ContractName;

use brutex_core::price::Paisa;

/// A ratio above this is a failure. Thousandths, so the comparison is integer.
///
/// 3.0x — `docs/04-invariants.md`, the shared-CI number.
const CEILING_PERMILLE: u128 = 3_000;

/// How many times each measurement is repeated before the minimum is taken.
const TRIALS: u32 = 40;

/// The real row count of the sample lake file, and two magnitudes above it.
const SMALL: usize = 2_480;
const MEDIUM: usize = 24_800;
const LARGE: usize = 248_000;

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
fn ratio(label: &str, base_ps: u128, at_ps: u128) -> bool {
    if base_ps == 0 {
        println!("  {label:<48} UNMEASURABLE — the 1x baseline timed at zero");
        return false;
    }
    let permille = at_ps * 1_000 / base_ps;
    let ok = permille <= CEILING_PERMILLE;
    println!(
        "  {label:<48} {:>9} ps -> {:>9} ps   ratio {}.{:03}x  {}",
        base_ps,
        at_ps,
        permille / 1_000,
        permille % 1_000,
        if ok { "ok" } else { "BREACH" }
    );
    ok
}

/// A cash-shaped batch of `n` rows.
///
/// Built through the crate's own public constructor path so the bench measures
/// the shipped type and not a copy of it.
fn batch(n: usize) -> Batch {
    let idx: Vec<i64> = (0..n).map(|i| i64::try_from(i).unwrap_or(0)).collect();
    let paisa: Vec<Paisa> = idx.iter().map(|v| Paisa::from_raw(*v)).collect();
    Batch::from_cash_columns(
        idx.clone(),
        paisa.clone(),
        paisa.clone(),
        paisa.clone(),
        paisa,
        idx.clone(),
        idx,
    )
    .unwrap_or_else(|| {
        println!("BENCH SETUP FAILED — columns disagreed in length");
        std::process::exit(1)
    })
}

/// The claim: a row lookup does not get more expensive as the batch grows.
/// The per-row floor: the cheapest possible touch of the same batch.
///
/// # Why a ratio alone cannot see a regression
///
/// Every row here divides one lookup cost by another, and a UNIFORM slowdown
/// cancels in a quotient. An audit measured exactly that elsewhere in this
/// workspace: a mask operation **174x slower passed its crate's ratio rows at
/// 0.98x-1.00x**, because both legs moved together.
///
/// The denominator has to be something that cannot move when `row` does. This
/// reads the batch's own length and folds it — same struct, same pointer, no row
/// decode — so the quotient is "how many of the cheapest batch touches does one
/// row lookup cost".
fn floor_ps(b: &Batch) -> u128 {
    cost_ps(2_000, || {
        let n = black_box(b).len();
        black_box(n.wrapping_add(1))
    })
}

/// Prints one budget in floors and returns whether it held.
fn budget(label: &str, floor: u128, at_ps: u128, allowed: u128) -> bool {
    if floor == 0 {
        println!("  {label:<52} UNMEASURABLE — the floor timed at zero");
        return false;
    }
    let floors = at_ps.saturating_mul(1_000) / floor;
    let ok = floors <= allowed.saturating_mul(1_000);
    println!(
        "  {label:<52} {at_ps:>8} ps = {}.{:03} floors, budget {allowed}   {}",
        floors / 1_000,
        floors % 1_000,
        if ok { "ok" } else { "OVER BUDGET" }
    );
    ok
}

/// C-L-04 — one row lookup costs a bounded multiple of the per-row floor.
fn row_lookup_stays_within_its_budget() -> bool {
    /// Floors allowed per lookup.
    ///
    /// Measured, arm64 laptop, release, three consecutive runs: **10.071,
    /// 10.011, 10.011** floors, at a floor of 437–937 ps.
    ///
    /// The RATIO held at ten even though the floor itself moved **2.1x** between
    /// runs — which is the whole argument for measuring against a floor rather
    /// than a wall-clock number. Both legs move with the machine; what stays
    /// fixed is how many of one the other costs.
    ///
    /// **40**, sized on the worst observed with roughly 4x left over, and still
    /// refusing the 174x uniform regression by a factor of 43.
    ///
    /// A breach is NOT a batch-size dependence — C-L-01 is that row. It means
    /// every lookup got dearer at once, which a quotient cannot see.
    const ALLOWED: u128 = 40;

    let b = batch(LARGE);
    let floor = floor_ps(&b);
    println!("  the per-row floor is {floor} ps — one length read and an add");
    let at = cost_ps(2_000, || b.row(black_box(LARGE - 1)));
    budget(
        "C-L-04 row(last) against the per-row floor",
        floor,
        at,
        ALLOWED,
    )
}

fn row_lookup_is_constant_in_batch_size() -> bool {
    let small = batch(SMALL);
    let medium = batch(MEDIUM);
    let large = batch(LARGE);

    // Index 0, which any implementation makes cheap.
    let first_small = cost_ps(2_000, || small.row(black_box(0)));
    let first_large = cost_ps(2_000, || large.row(black_box(0)));

    // The last row, which is where a scan would show itself.
    let last_small = cost_ps(2_000, || small.row(black_box(SMALL - 1)));
    let last_medium = cost_ps(2_000, || medium.row(black_box(MEDIUM - 1)));
    let last_large = cost_ps(2_000, || large.row(black_box(LARGE - 1)));

    let mut ok = true;
    ok &= ratio(
        "C-L-01 row(0): 2,480 rows -> 248,000 rows",
        first_small,
        first_large,
    );
    ok &= ratio(
        "C-L-01 row(last): 2,480 -> 24,800 rows",
        last_small,
        last_medium,
    );
    ok &= ratio(
        "C-L-01 row(last): 2,480 -> 248,000 rows",
        last_small,
        last_large,
    );
    // The sharpest form of the same question: inside ONE large batch, the last
    // row must cost what the first row costs. A scan would be 248,000x here.
    ok &= ratio(
        "C-L-01 row(first) -> row(last), within 248,000",
        first_large,
        last_large,
    );
    ok
}

/// Iteration must stay linear: cost per row, not cost per row per row.
fn iteration_is_linear_per_row() -> bool {
    let small = batch(SMALL);
    let large = batch(LARGE);

    let per_row_small = cost_ps(20, || small.iter().count()) / u128::try_from(SMALL).unwrap_or(1);
    let per_row_large = cost_ps(20, || large.iter().count()) / u128::try_from(LARGE).unwrap_or(1);

    ratio(
        "C-L-02 cost PER ROW of a full walk: 2,480 -> 248,000",
        per_row_small,
        per_row_large,
    )
}

/// Parsing a contract name does not depend on how long the name is.
fn contract_parse_does_not_scan_the_name() -> bool {
    // A real name, and a refused one that is far longer. The refusal must not
    // cost more than the accept — a parser that scanned the whole string
    // before deciding would show it here.
    let real = "NSE-NIFTY-01Apr20-10000-CE";
    let long_refused = format!("NSE-NIFTY-01Apr20-{}-CE", "9".repeat(4096));

    let short = cost_ps(2_000, || ContractName::parse(black_box(real)));
    let long = cost_ps(2_000, || ContractName::parse(black_box(&long_refused)));

    ratio(
        "C-L-03 contract parse: 26 byte name -> 4 KiB name",
        short,
        long,
    )
}

fn main() {
    println!("gate 8 — crates/lake, ceiling {CEILING_PERMILLE} permille");
    let mut ok = true;
    ok &= row_lookup_is_constant_in_batch_size();
    ok &= row_lookup_stays_within_its_budget();
    ok &= iteration_is_linear_per_row();
    ok &= contract_parse_does_not_scan_the_name();
    if ok {
        println!("all ratios within the ceiling");
    } else {
        println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
        std::process::exit(1);
    }
}
