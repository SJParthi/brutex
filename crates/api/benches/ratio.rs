//! Gate 8 for `crates/api` — rendering one page costs the same whatever the
//! size of the instrument set behind it.
//!
//! # The regression this exists to catch
//!
//! `docs/07-o1-architecture.md` layer 12 was marked BUILT with the words
//! "a fixed row cap with paging. NEVER O(universe)". Only the *rendered rows*
//! were capped. Every request re-folded the whole `by_key` map to compute the
//! universe pill counts, re-filtered it into a fresh `Vec`, and re-sorted that
//! vector — so the cost of drawing 200 rows tracked the size of the map behind
//! them. An audit of a running server measured 3.569 ms at 2,787 instruments
//! and 124.916 ms at 50,000, **rendering exactly 200 rows both times**, a
//! marginal 62.5 ns per instrument per request.
//!
//! # What is asserted, and why it is not one number
//!
//! A page at 2 instruments draws 2 rows and a page at 2,787 draws 200. Their
//! costs are *supposed* to differ — by rows, which is the bound the page is
//! allowed to have. Comparing them raw would assert the wrong thing and pass
//! the wrong code. So three gates, and each one isolates a different way the
//! defect could come back:
//!
//! | Gate | Compares | Catches |
//! |---|---|---|
//! | [`RATIO_CEILING_PERMILLE`] | 2,787 vs 50,000, both drawing 200 rows | any per-request whole-universe pass |
//! | [`MARGINAL_CEILING_PS`] | the slope between those two points | the audit's 62.5 ns per instrument, directly |
//! | [`PER_ROW_CEILING_PERMILLE`] | cost **per rendered row** at 2 vs at 50,000 | a page that got flat by drawing less |
//!
//! The absolute numbers move between machines; the shape does not, and the
//! shape is what these assert.
//!
//! See `crates/store/benches/ratio.rs` for why there is no benchmarking
//! framework and why every ratio here is integer arithmetic.
//!
//! # One sub-module per route family, each owned by the package that builds it
//!
//! The pages above are this file's own. The /db routes get a file each under
//! `benches/ratio/`, called from [`main`]: `census` (the census probe, WP-30),
//! `window` (the `/bars/window.json` seek, WP-31), `spot` (the `/spot.json`
//! probe, WP-32) and `sources` (the All sources routes, WP-37, not built yet
//! and said so). The WP-01 seam created them so each package adds its rows to
//! its own file and never to this one. The month fixture they share is below.
//!
//! Each is mounted by `#[path]`: this file is the bench's crate root, and a
//! crate root's bare `mod census;` resolves beside it, to `benches/census.rs`,
//! where cargo would also take it for a bench target of its own.

#[path = "ratio/census.rs"]
mod census;
#[path = "ratio/gaps.rs"]
mod gaps;
#[path = "ratio/logs_dirs.rs"]
mod logs_dirs;
#[path = "ratio/sources.rs"]
mod sources;
#[path = "ratio/spot.rs"]
mod spot;
#[path = "ratio/window.rs"]
mod window;

use std::hint::black_box;
use std::time::Instant;

use api::merge::{Entry, Merged};
use api::server::{Read, dashboard_html, instruments_html_from};
use brutex_core::instrument::{Exchange, InstrumentKey, Kind, Segment};
use brutex_core::isin::Isin;
use brutex_core::symbol::Symbol;
use brutex_core::universe::Universe;
use brutex_core::vendor::{Vendor, VendorSet};

/// A ratio above this is a failure, between two sizes that draw the same
/// number of rows. Thousandths, so the comparison is integer.
///
/// 3.0× — `docs/04-invariants.md`, the shared-CI number.
const RATIO_CEILING_PERMILLE: u128 = 3_000;

/// The most a request may cost per extra instrument in the universe, in
/// picoseconds.
///
/// 1,000 ps = 1 ns. The audit measured **62,500 ps** per instrument per
/// request against the pre-D-0042 code, so this is 62× below the defect and
/// still far above measurement noise on a flat implementation.
///
/// This constant **is** `C-15`'s ceiling, and `api::bench::gate` is where it is
/// asserted — the `C-15 … marginal` lines this file prints.
const MARGINAL_CEILING_PS: u128 = 1_000;

/// The most the cost **per rendered row** may grow between the smallest and
/// the largest universe. Thousandths.
///
/// Higher than the plain ratio because a 2-row page pays the page's fixed
/// chrome — nav, hero, pills, notes, table head — across 2 rows instead of
/// 200, so its per-row figure is legitimately the larger one. This gate is
/// here to catch the opposite: a large universe costing more per row.
const PER_ROW_CEILING_PERMILLE: u128 = 3_000;

/// How many times each measurement is repeated before the minimum is taken.
///
/// Lower than the 60 `crates/core` uses because one call here renders 200 rows
/// of HTML rather than decoding one row.
const TRIALS: u32 = 8;

/// The three sizes the audit measured, so the printed table is comparable.
///
/// `SMALL` draws fewer than a full page; `BASE` and `BIG` both draw a full 200
/// rows under every pill, which is what makes their ratio mean something.
const SMALL: usize = 2;
/// The real universe on the day of the audit.
const BASE: usize = 2_787;
/// Eighteen times the real universe.
const BIG: usize = 50_000;

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

/// Prints one ratio and returns whether it stayed under its ceiling.
fn ratio(label: &str, base_ps: u128, at_ps: u128, ceiling: u128) -> bool {
    if base_ps == 0 {
        println!("  {label:<46} UNMEASURABLE — the 1x baseline timed at zero");
        return false;
    }
    let permille = at_ps * 1_000 / base_ps;
    let ok = permille <= ceiling;
    println!(
        "  {label:<46} {:>10} ps -> {:>10} ps  ratio {}.{:03}x  {}",
        base_ps,
        at_ps,
        permille / 1_000,
        permille % 1_000,
        if ok { "ok" } else { "BREACH" }
    );
    ok
}

/// A bench may not `unwrap`, and `S0000000` is not a symbol that can fail.
/// If one ever is, say so and stop rather than measure nothing.
fn refuse(what: &str) -> ! {
    println!("  {what} was refused — the bench measured nothing");
    std::process::exit(1);
}

/// The three month sizes the route rows time, in bars: 1x / 10x / 100x.
///
/// One-second bars, because 102,400 of them fit one month (under 29 hours) and
/// a one-minute month holds at most 44,640: the store's month admission
/// refuses the rest (D-0915).
///
/// **1x is the smallest month the store indexes**,
/// `store::time_index::INDEX_MIN_ROWS` bars (D-4933). A month under it is
/// given no `.tix` and no `.srt`: its seek bisects and its value sort orders
/// its own rows, which are not the paths these rows time. At 1,000, this
/// array's first size before that rule, the 1x month took those paths while
/// 10x and 100x read the indexes, so every ratio compared a bisection or an
/// in-memory sort with an index read and passed whatever the index cost.
/// [`month_file`] refuses a month that is not indexed, so a threshold raised
/// past these sizes stops the bench rather than re-routing it.
const MONTH_SIZES: [usize; 3] = [1_024, 10_240, 102_400];

/// 2024-06-01 00:00 IST in epoch microseconds, where every fixture bar starts.
const JUNE_2024_IST_START: i64 = 1_717_180_200_000_000;

/// The month every route fixture is filed under.
fn bench_month() -> store::path::YearMonth {
    match store::path::YearMonth::new(2024, 6) {
        Ok(month) => month,
        Err(_) => refuse("June 2024 is a real month"),
    }
}

/// The stamp of fixture bar `i`: one second apart from the month's start.
fn second(i: usize) -> i64 {
    let raw = i64::try_from(i).unwrap_or(i64::MAX);
    JUNE_2024_IST_START.saturating_add(raw.saturating_mul(1_000_000))
}

/// One one-second month of `n` bars on disk -- GDFL, NSE INDEX NIFTY, June
/// 2024 -- opened for reading, and the store root that owns it.
///
/// The root is returned so it outlives the file, and each caller removes it
/// when it is done. Appended in one batch, so the setup is out of every figure.
fn month_file(name: &str, n: usize) -> (store::file::BarFile, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "brutex-api-bench-{name}-{n}-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&root);
    if let Err(why) = std::fs::create_dir_all(&root) {
        refuse(&format!("the bench store root would not create: {why}"));
    }
    let Ok(path) = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: store::path::Timeframe::SECOND_1,
        month: bench_month(),
        file: store::path::FileKind::Bars,
    }) else {
        refuse("the bench path is a legal one")
    };
    // THE ID `pull::ingest` STAMPS AND `api::bars::open` CHECKS: the low 32
    // bits of the symbol's 64-bit FNV-1a. A month created under any other id is
    // refused by every reader on reopen, so the window seek would time a
    // refusal rather than a page.
    let Ok(symbol_id) = u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX))
    else {
        refuse("the low 32 bits of a hash fit 32 bits")
    };
    let mut file = match store::file::BarFile::open_or_create(&root, path, symbol_id) {
        Ok(file) => file,
        Err(why) => refuse(&format!("the bench month would not open: {why}")),
    };
    let batch: Vec<store::format::Bar> = (0..n)
        .map(|i| {
            let raw = i64::try_from(i).unwrap_or(0);
            store::format::Bar {
                ts_micros: second(i),
                open: 2_000_000 + raw,
                high: 2_000_100 + raw,
                low: 1_999_900 + raw,
                close: 2_000_050 + raw,
                volume: 1_000,
                open_interest: store::format::OI_NULL,
            }
        })
        .collect();
    if let Err(why) = file.append(&batch) {
        refuse(&format!("the bench month would not fill: {why}"));
    }
    // BOTH INDEXES OR NO MEASUREMENT (D-4933): a month the store gave no
    // `.tix` or no `.srt` would time the bisection or the in-memory sort.
    let lookup = file.time_lookup();
    let srt = path
        .with_file(store::path::FileKind::SortIndex)
        .to_path_buf(&root);
    if lookup != store::file::TimeLookup::Indexed || !srt.is_file() {
        refuse(&format!(
            "a {n}-bar bench month with its .tix and .srt (found {lookup:?}, .srt {})",
            if srt.is_file() { "present" } else { "absent" }
        ));
    }
    (file, root)
}

/// One synthetic instrument key.
fn key(n: usize) -> InstrumentKey {
    InstrumentKey {
        exchange: Exchange::Nse,
        segment: Segment::Cash,
        // A symbol holds 24 bytes; `S` plus seven digits is 8.
        underlying: Symbol::new(&format!("S{n:07}")).unwrap_or_else(|_| refuse("a symbol")),
        kind: Kind::Equity,
    }
}

/// A universe of `n` instruments, every one of them tracked so the page
/// actually renders rows.
///
/// Membership is set on the [`Entry`] directly rather than derived from
/// `universe::of_instrument`, because a synthetic symbol is in no real index
/// and a page that renders zero rows measures nothing.
///
/// Every fourth instrument is an F&O underlying and every tenth is an index,
/// so **every pill draws a full 200-row page** at [`BASE`] and at [`BIG`].
/// That is deliberate: a pill that ran out of rows at one size and not the
/// other would produce a ratio that measured the row count, not the universe.
fn read_of(n: usize) -> Read {
    let mut by_key = std::collections::HashMap::with_capacity(n);
    for i in 0..n {
        let mut u = Universe::TOTAL_MARKET;
        if i % 4 == 0 {
            u = u.union(Universe::FNO);
        }
        if i % 10 == 0 {
            u = u.union(Universe::INDEX);
        }
        by_key.insert(
            key(i),
            Entry {
                ids: [None; brutex_core::vendor::Vendor::ALL.len()],
                vendors: VendorSet::EMPTY.with(Vendor::Groww).with(Vendor::Dhan),
                // A body that fails the check digit is `None`, which is a real
                // state a row renders. Either way the ISIN column has content.
                isin: Isin::new(&format!("INE{:09}", i % 1_000_000_000))
                    .ok()
                    .map(|x| (Vendor::Groww, x)),
                conflict: None,
                universe: u,
                ..Entry::default()
            },
        );
    }
    Read::new(
        Merged {
            by_key,
            conflicts: Vec::new(),
            eligibility: Vec::new(),
            // Both brokers' masters, matching the comment two lines below.
            contributed: vec![
                brutex_core::vendor::Vendor::Groww,
                brutex_core::vendor::Vendor::Dhan,
            ],
            ..Merged::default()
        },
        Vec::new(),
        // unread: every vendor's master was read, by construction.
        Vec::new(),
        0,
        // unreadable, then non_routine: a synthetic bench universe decodes
        // cleanly by construction, so this harness measures render cost on a
        // CLEAN universe and neither degrade gate fires.
        0,
        0,
    )
}

/// The three universes, built once. Rebuilding per measurement would time the
/// build, which is exactly the work D-0042 moved *out* of the request.
struct Fixtures {
    small: Read,
    base: Read,
    big: Read,
}

/// One page measured at all three sizes, with the row count it drew.
struct Measured {
    small_ps: u128,
    base_ps: u128,
    big_ps: u128,
    small_rows: usize,
    big_rows: usize,
}

/// How many `<tr` a page carries, less the header row.
fn drawn(html: &str) -> usize {
    html.matches("<tr").count().saturating_sub(1)
}

/// Measures one page at all three sizes.
fn measure(f: &Fixtures, reps: u32, page: impl Fn(&Read) -> String) -> Measured {
    Measured {
        small_ps: cost_ps(reps, || page(black_box(&f.small))),
        base_ps: cost_ps(reps, || page(black_box(&f.base))),
        big_ps: cost_ps(reps, || page(black_box(&f.big))),
        small_rows: drawn(&page(&f.small)),
        big_rows: drawn(&page(&f.big)),
    }
}

/// Applies all three gates to one measurement and prints every line.
fn gate(label: &str, m: &Measured) -> bool {
    let mut ok = ratio(
        &format!("C-14 {label} {BASE}->{BIG}"),
        m.base_ps,
        m.big_ps,
        RATIO_CEILING_PERMILLE,
    );

    // THE AUDIT'S NUMBER, DIRECTLY. Slope between two points that draw the
    // same rows, so every picosecond of it is universe size and nothing else.
    // Saturating: an implementation that got *faster* at the larger size has a
    // slope of zero, not a negative one.
    let marginal = m.big_ps.saturating_sub(m.base_ps) / (BIG - BASE) as u128;
    let slope_ok = marginal <= MARGINAL_CEILING_PS;
    println!(
        "  {:<46} {marginal:>10} ps per instrument per request       {}",
        format!("C-15 {label} marginal"),
        if slope_ok { "ok" } else { "BREACH" }
    );
    ok &= slope_ok;

    // AND THE PAGE STILL DREW ITS ROWS. Speed alone would pass an
    // implementation that got flat by rendering nothing, which is the same
    // class of mistake as capping the rows and calling the page constant.
    if m.small_rows == 0 || m.big_rows == 0 {
        println!(
            "  {:<46} drew {} and {} rows — MEASURED NOTHING",
            format!("C-17 {label} rows"),
            m.small_rows,
            m.big_rows
        );
        return false;
    }
    let per_row_small = m.small_ps / m.small_rows as u128;
    let per_row_big = m.big_ps / m.big_rows as u128;
    ok &= ratio(
        &format!("C-17 {label} per row {SMALL}->{BIG}"),
        per_row_small,
        per_row_big,
        PER_ROW_CEILING_PERMILLE,
    );
    println!(
        "      rows drawn: {} at n={SMALL}, {} at n={BIG}; raw n={SMALL} {} ps",
        m.small_rows, m.big_rows, m.small_ps
    );
    ok
}

/// C-16 — the dashboard costs the same at 2 and at 50,000 instruments.
///
/// The dashboard draws no rows at all, so the raw ratio between the smallest
/// and the largest universe is the whole story here and no per-row correction
/// applies.
/// The per-render floor: the cheapest possible touch of the same `Read`.
///
/// # Why a ratio alone cannot see a regression
///
/// Every row here divides one render cost by another, and a UNIFORM slowdown
/// cancels in a quotient. An audit measured exactly that elsewhere in this
/// workspace: a mask operation **174x slower passed its crate's ratio rows at
/// 0.98x-1.00x**, because both legs moved together. This was the last of the
/// thirteen crates without a floor-relative budget.
///
/// The denominator has to be something that cannot move when a page render
/// does. This formats a single integer into a `String` — one allocation and one
/// integer-to-text conversion, the smallest unit of work any HTML page here is
/// built from. The quotient is "how many one-number writes does a whole page
/// cost".
fn floor_ps() -> u128 {
    use std::fmt::Write as _;
    cost_ps(2_000, || {
        let mut out = String::with_capacity(8);
        let _ignored = write!(out, "{}", black_box(50_000_u32));
        black_box(out)
    })
}

/// Prints one budget in floors and returns whether it held.
fn budget(label: &str, floor: u128, at_ps: u128, allowed: u128) -> bool {
    if floor == 0 {
        println!("  {label:<44} UNMEASURABLE — the floor timed at zero");
        return false;
    }
    let floors = at_ps.saturating_mul(1_000) / floor;
    let ok = floors <= allowed.saturating_mul(1_000);
    println!(
        "  {label:<44} {at_ps:>9} ps = {}.{:03} floors, budget {allowed}   {}",
        floors / 1_000,
        floors % 1_000,
        if ok { "ok" } else { "OVER BUDGET" }
    );
    ok
}

/// C-27 — one dashboard render costs a bounded multiple of the per-render floor.
fn the_dashboard_stays_within_its_budget(f: &Fixtures) -> bool {
    /// Floors allowed per render.
    ///
    /// Measured, arm64 laptop, release, three consecutive runs: **529.464,
    /// 493.599, 528.448** floors, at a floor of 20,771–21,041 ps. A 1.07x
    /// spread.
    ///
    /// **2,100**, sized on the worst observed with roughly 4x left over — the
    /// same rule the other ten budgets apply. It still refuses the 174x uniform
    /// regression this file's ratios would report as ok: such a render would
    /// read about 92,000 floors and be refused by a factor of 43.
    ///
    /// **What the magnitude says**, and it is worth stating: a whole instruments
    /// page at 50,000 rows costs about five hundred single-number writes. That
    /// is a page, not a per-row cost — C-14 and C-15 are the rows that bound the
    /// per-row and marginal figures. This one exists so that all three of them
    /// getting dearer at once is still visible, which no quotient between them
    /// can show.
    const ALLOWED: u128 = 2_100;

    let floor = floor_ps();
    println!("  the per-render floor is {floor} ps — one number written to a String");
    let at = cost_ps(50, || dashboard_html(black_box(&f.big)));
    budget(
        "C-27 dashboard against the per-render floor",
        floor,
        at,
        ALLOWED,
    )
}

fn the_dashboard_is_flat(f: &Fixtures) -> bool {
    let small = cost_ps(50, || dashboard_html(black_box(&f.small)));
    let base = cost_ps(50, || dashboard_html(black_box(&f.base)));
    let big = cost_ps(50, || dashboard_html(black_box(&f.big)));
    let mut ok = ratio(
        &format!("C-16 dashboard {SMALL}->{BASE}"),
        small,
        base,
        RATIO_CEILING_PERMILLE,
    );
    ok &= ratio(
        &format!("C-16 dashboard {SMALL}->{BIG}"),
        small,
        big,
        RATIO_CEILING_PERMILLE,
    );
    ok
}

/// C-14 — every sort order and every pill the UI offers is flat.
///
/// No combination is left unmeasured, because the defect was invisible
/// precisely where nobody looked.
fn every_order_and_pill_is_flat(f: &Fixtures) -> bool {
    let mut ok = true;
    for sort in [
        "", "symbol", "isin", "universe", "kind", "vendors", "-symbol",
    ] {
        for pill in ["", "fno", "ntm", "idx"] {
            let m = measure(f, 20, |r| {
                instruments_html_from(r, "", sort, false, pill, 0)
            });
            ok &= gate(&format!("sort={sort:?} pill={pill:?}"), &m);
        }
    }
    ok
}

/// C-14 — the escape hatch and a deep page are flat too.
///
/// `all=true` lifts the universe filter and `?page=9999` clamps to the last
/// page. Both were a separate `Vec` build and a separate sort before D-0042,
/// and the deep page additionally paid a `skip()` over everything ahead of it.
fn the_hatch_and_the_last_page_are_flat(f: &Fixtures) -> bool {
    let mut ok = true;
    for (label, all, page) in [("hatch all=true", true, 0), ("last page", false, 9_999)] {
        let m = measure(f, 20, |r| instruments_html_from(r, "", "", all, "", page));
        ok &= gate(label, &m);
    }
    ok
}

/// Search is the one thing that is **not** constant per rendered row, and it
/// is measured here rather than left silent. `docs/06-limits.md` §24.
///
/// Printed, never asserted: a substring that matches most of the universe must
/// look at most of the universe, and no index makes that untrue.
///
/// These synthetic names are the **worst possible** input for the trigram
/// index — every name is `NSE-S` followed by seven digits, so almost every
/// name carries almost every trigram and the index narrows nothing. Real
/// symbols share few trigrams. The figures below are therefore an upper bound
/// on search cost, not a typical one, and they are reported that way.
fn search_is_measured_and_named(f: &Fixtures) {
    for (needle, what) in [
        ("S0000001", "8 bytes, trigram-narrowed"),
        ("S0", "2 bytes — no trigram exists to narrow with"),
        ("S00", "a trigram nearly every synthetic name carries"),
    ] {
        for (n, read) in [(SMALL, &f.small), (BASE, &f.base), (BIG, &f.big)] {
            let ps = cost_ps(5, || {
                instruments_html_from(black_box(read), needle, "", false, "", 0)
            });
            println!(
                "  {:<46} {ps:>10} ps   ({what})",
                format!("§24 search {needle:?} n={n}")
            );
        }
    }
}

fn main() {
    println!("gate 8 — crates/api");
    println!(
        "  ratio ceiling {RATIO_CEILING_PERMILLE} permille · \
         marginal ceiling {MARGINAL_CEILING_PS} ps/instrument"
    );
    let started = Instant::now();
    let f = Fixtures {
        small: read_of(SMALL),
        base: read_of(BASE),
        big: read_of(BIG),
    };
    println!(
        "  built universes of {SMALL}, {BASE} and {BIG} once, in {} ms — \
         every figure below is per REQUEST",
        started.elapsed().as_millis()
    );
    let mut ok = true;
    ok &= the_dashboard_is_flat(&f);
    ok &= the_dashboard_stays_within_its_budget(&f);
    ok &= every_order_and_pill_is_flat(&f);
    ok &= the_hatch_and_the_last_page_are_flat(&f);
    search_is_measured_and_named(&f);
    ok &= census::probes_are_flat();
    ok &= window::seeks_are_flat();
    ok &= spot::probes_are_flat();
    ok &= gaps::answers_are_flat();
    ok &= sources::pages_are_flat();
    ok &= logs_dirs::reads_are_flat("C-LG-01", "C-LG-02");
    if ok {
        println!("all ratios within the ceiling");
    } else {
        println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
        std::process::exit(1);
    }
}
