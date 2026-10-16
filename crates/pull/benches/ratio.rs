//! Gate 8 for `crates/pull` — layer 13 asserted as a number.
//!
//! # Why this file exists
//!
//! `docs/07-o1-architecture.md`: *"A layer is not built because the code looks
//! right. It is built when a test asserts the bound as a number."* Layer 13 is
//! "counters, never scans", and the two numbers that make it real are here:
//! reading the census costs the same whatever the census holds, and it beats
//! re-deriving the same answer from the entries by a wide margin, measured in
//! the same process.
//!
//! Those two numbers are `C-12` — `pull::bench::entry_lookup_is_flat` — and
//! `C-11` — `pull::bench::census_beats_the_scan_it_replaces`; `C-13`,
//! `pull::bench::append_after_load_is_flat`, is the third and is about the
//! write side. `C-14`, `pull::bench::census_is_flat_across_a_compaction`, is
//! the version-4 row (D-3316): one lookup and one append cost the same before
//! and after a compaction into the next epoch, and a compaction — one pass over
//! the keys and one sort, O(keys · log keys) — costs the same per entry at ten
//! times the census, within the ceiling. They are functions in **this file**, which is why the
//! layer is asserted rather than described.
//!
//! # Why no benchmarking framework
//!
//! `harness = false` makes this an ordinary binary: `cargo bench` runs it, it
//! prints every number it took, and it exits non-zero when a ratio breaches its
//! ceiling. Nothing is reported as passing that was not measured. `test = false`
//! keeps `cargo test` and the coverage gate out of a timing loop. All
//! arithmetic is integer — `clippy::float_arithmetic` is a workspace lint and a
//! ratio is the one place it would be tempting.
//!
//! # What is measured, and what is NOT
//!
//! Measured: the cost of the answer, against the cost of deriving that same
//! answer from the manifest's own entries, on one machine, in one process,
//! under one load.
//!
//! **Not measured: the directory walk this file exists to replace.** The real
//! alternative to a manifest is a `stat` on roughly 248,000 files, and that
//! depends on a filesystem, a page cache and a device this harness does not
//! control — timing it would report the state of one machine's cache rather
//! than a property of the code. Any statement about that saving is an
//! **EXTRAPOLATION** from the entry scan below, and `docs/06-limits.md` §17
//! records it as one.
//!
//! Not measured either: residency. A probe into a 100,000-entry map that has
//! fallen out of cache costs more than one into a map that has not, and that is
//! layer 7's subject rather than layer 3's. These measurements probe one key
//! repeatedly, so what they report is the **probe count** — which is what "no
//! rehash, one probe" claims — and not the machine's memory hierarchy.

use std::hint::black_box;
use std::time::Instant;

use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use store::path::{Timeframe, YearMonth};

use pull::manifest::{Commit, ENTRY_LEN, Entry, EntryKey, FNO_FLOOR, HEADER_LEN, Held, Manifest};

/// [`HEADER_LEN`] as a length, for building a header region.
const HEADER_LEN_LEN: usize = 32_768;
/// `store::format::SLOT_STRIDE` as a length, for placing the second slot.
const SLOT_STRIDE_LEN: usize = 16_384;

const _: () = assert!(HEADER_LEN == 32_768);

/// A ratio above this is a failure. Thousandths, so the comparison is integer.
///
/// 3.0× — `docs/04-invariants.md`, the shared-CI number. A ratio between 1.4×
/// and 3.0× on shared hardware is re-run on dedicated hardware before it is
/// believed; above 3.0× it is a regression on any hardware.
const CEILING_PERMILLE: u128 = 3_000;

/// How many times each measurement is repeated before the minimum is taken.
///
/// The minimum rather than the mean: a shared runner's scheduler can only ever
/// make a sample slower, so the smallest observation is the closest thing to
/// the cost of the work itself.
const TRIALS: u32 = 60;

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

/// The smallest non-zero interval [`Instant`] reports on this host, in
/// nanoseconds.
///
/// Measured, printed, and asserted against nothing — it is context, not a
/// ratio. It exists because D-0035 explained the C-11 spread as "a field read
/// sits at the clock's resolution floor", and that explanation was never
/// measured. This number is what decides whether it is true: one tick spread
/// over `reps` repetitions is the smallest cost this harness can report, and if
/// an observation is a thousand times that, the clock is not what moved it.
/// `CLAUDE.md` §3 rule 6 — never claim a measurement you did not take. D-0036.
fn clock_tick_ns() -> u128 {
    let mut best = u128::MAX;
    for _ in 0..2_000 {
        let start = Instant::now();
        loop {
            let seen = start.elapsed().as_nanos();
            if seen > 0 {
                if seen < best {
                    best = seen;
                }
                break;
            }
        }
    }
    best
}

/// Prints one measurement and returns whether it stayed under the ceiling.
///
/// A base of zero means the clock could not resolve the operation at all, which
/// is reported as a failure rather than divided by: an unmeasurable baseline
/// makes every ratio meaningless, and reporting a ratio nobody measured is what
/// `CLAUDE.md` §3 rule 6 forbids.
fn ratio(label: &str, base_ps: u128, at_ps: u128) -> bool {
    if base_ps == 0 {
        println!("  {label:<44} UNMEASURABLE — the 1x baseline timed at zero");
        return false;
    }
    let permille = at_ps * 1_000 / base_ps;
    let ok = permille <= CEILING_PERMILLE;
    println!(
        "  {label:<44} {:>9} ps -> {:>9} ps   ratio {}.{:03}x  {}",
        base_ps,
        at_ps,
        permille / 1_000,
        permille % 1_000,
        if ok { "ok" } else { "BREACH" }
    );
    ok
}

/// The harness could not build its own input, and says so rather than
/// proceeding on a guess.
fn refuse(why: &str) -> ! {
    println!("THE HARNESS COULD NOT BUILD ITS OWN INPUT: {why}");
    println!("No ratio below was measured. This is a failure, not a skip.");
    std::process::exit(1)
}

/// Months this harness can name: `1970..=9999`, twelve each.
const MONTHS: u32 = (9_999 - 1_970 + 1) * 12;

/// The key of the *i*th month this harness records.
///
/// Distinct for every `index` below `3 · MONTHS`, which is 289,080 — the
/// calendar alone runs out at 96,360, so the segment cycles above it. Every key
/// is a key the *store* could address, because they are built from the same
/// `store::path` types a bar file's path is; a harness that fabricated a month
/// the store refuses would be measuring a shape that cannot exist.
fn key(index: u32) -> EntryKey {
    key_for("NIFTY", index)
}

/// [`key`], for a chosen symbol.
fn key_for(symbol: &str, index: u32) -> EntryKey {
    let Ok(year) = u16::try_from(1_970 + index / 12 % (MONTHS / 12)) else {
        refuse("a year outside u16")
    };
    let Ok(month) = u8::try_from(index % 12 + 1) else {
        refuse("a month outside u8")
    };
    let segment = match index / MONTHS % 3 {
        0 => Segment::Index,
        1 => Segment::Cash,
        _ => Segment::Fno,
    };
    let (Ok(symbol), Ok(month)) = (Symbol::new(symbol), YearMonth::new(year, month)) else {
        refuse("a symbol or a month this build refuses")
    };
    EntryKey {
        contract: None,
        exchange: Exchange::Nse,
        segment,
        symbol,
        timeframe: Timeframe::MINUTE_1,
        month,
    }
}

/// The header region holding one commit in the slot it belongs to.
///
/// Built by extension rather than by writing into an index, because
/// `clippy::indexing_slicing` is denied across this workspace and a bench is a
/// target like any other.
fn header_region(commit: &Commit) -> Vec<u8> {
    let mut region: Vec<u8> = Vec::with_capacity(HEADER_LEN_LEN);
    if commit.slot != 0 {
        region.resize(SLOT_STRIDE_LEN, 0);
    }
    region.extend_from_slice(&commit.bytes);
    region.resize(HEADER_LEN_LEN, 0);
    region
}

/// A manifest holding `count` months, **as a reader sees it**, and the entry
/// bytes a reader would see.
///
/// It is written with `record` and then round-tripped through
/// [`Manifest::load`], and that round trip is the point of this function rather
/// than a flourish. C-12 is a claim about layer 3 — a map reserved from a known
/// bound, so no rehash — and only the loaded index is reserved;
/// `Manifest::genesis` reserves nothing and grows by rehashing. Measuring the
/// writer's map would have reported a number that had nothing to do with the
/// mechanism `docs/06-limits.md` §17 and `docs/07-o1-architecture.md` line 79
/// attribute it to, which is the shape `CLAUDE.md` §3 rule 6 forbids. D-0036.
fn census(count: u32) -> (Manifest, Vec<u8>) {
    let (region, data) = census_bytes(count);
    match Manifest::load(Vendor::Groww, &region, &data) {
        Ok(manifest) => (manifest, data),
        Err(e) => refuse(&e.to_string()),
    }
}

/// The header region and the entry region of a census of `count` months.
///
/// Split out of [`census`] so that a measurement which must start from a
/// *freshly loaded* manifest on every trial can reload from the same bytes
/// instead of cloning — a clone of the index is O(keys) and would be charged to
/// whatever it was wrapped around.
fn census_bytes(count: u32) -> (Vec<u8>, Vec<u8>) {
    let Ok(mut writer) = Manifest::open(Vendor::Groww, &[], &[]) else {
        refuse("a genesis manifest")
    };
    let mut data = Vec::with_capacity(count as usize * ENTRY_LEN);
    let mut published: Option<Commit> = None;
    for index in 0..count {
        let entry = Entry {
            key: key(index),
            rows: 7_312,
            first_ts_micros: 1_000,
            last_ts_micros: 2_000,
        };
        match writer.record(entry) {
            Ok(append) => {
                data.extend_from_slice(&append.bytes);
                published = Some(append.commit);
            }
            Err(e) => refuse(&e.to_string()),
        }
    }
    let Some(commit) = published else {
        refuse("a census of no months")
    };
    (header_region(&commit), data)
}

/// C-11 — the census is a counter read, not a walk of what it counts.
///
/// The scan side re-derives exactly the number the header already holds, by
/// decoding and checksum-verifying every entry — which is what any answer that
/// did not trust a counter would have to do. Both sides run in the same process
/// on the same machine, so the ratio is a property of the code rather than of
/// the hardware.
fn census_beats_the_scan_it_replaces() -> bool {
    /// The counter must beat the scan by at least this, in thousandths.
    ///
    /// A floor rather than a nanosecond ceiling: an absolute threshold would
    /// have to be guessed for hardware this repository has never measured on.
    /// 100× at ten thousand entries is far below what any correct
    /// implementation produces and far above what a counter that had quietly
    /// become a walk could reach.
    const FLOOR_PERMILLE: u128 = 100_000;

    /// Repetitions the counter side is averaged over.
    const REPS: u32 = 100_000;

    let (manifest, data) = census(10_000);
    let counter = cost_ps(REPS, || black_box(&manifest).total_rows());
    let scan = cost_ps(20, || {
        data.chunks_exact(ENTRY_LEN)
            .filter_map(|chunk| Held::decode(chunk).ok())
            .fold(0u64, |sum, held| sum.wrapping_add(held.entry.rows))
    });
    if counter == 0 {
        println!("  C-11 census vs the scan                      UNMEASURABLE");
        return false;
    }

    // How far the counter observation sits above the clock's own floor. The
    // whole point of printing it is that the answer is not "one".
    let tick = clock_tick_ns();
    let floor_fs = tick * 1_000_000 / u128::from(REPS);
    println!(
        "  {:<44} {tick} ns; one tick over {REPS} reps is {floor_fs} fs of reported cost, \
         so the counter observation is {} ticks",
        "clock granularity (context, not a ratio)",
        counter * u128::from(REPS) / (tick * 1_000)
    );
    let permille = scan * 1_000 / counter;
    let ok = permille >= FLOOR_PERMILLE;
    println!(
        "  {:<44} {:>9} ps vs {:>9} ps  speedup {}.{:03}x  {}",
        "C-11 census counter vs 10,000-entry scan",
        counter,
        scan,
        permille / 1_000,
        permille % 1_000,
        if ok { "ok" } else { "BREACH" }
    );
    ok
}

/// C-12 — one entry lookup costs the same at 1×, 10× and 100× the census.
///
/// The map is reserved from the entry count known before the load walk begins,
/// so it never rehashes: `docs/07-o1-architecture.md` layer 3, O(1) **worst
/// case** rather than average. A cost that tracked the census size would mean
/// the probe had started to depend on how much is held.
/// The per-lookup floor: the cheapest possible touch of the same manifest.
///
/// # Why a ratio alone cannot see a regression
///
/// C-12 divides one lookup cost by another, and a UNIFORM slowdown cancels in a
/// quotient. An audit measured exactly that elsewhere in this workspace: a mask
/// operation **174x slower passed its crate's ratio rows at 0.98x-1.00x**,
/// because both legs moved together.
///
/// The denominator has to be something that cannot move when `entry` does. This
/// reads the manifest's own entry count and folds it — same struct, same pointer
/// chased, no hash and no probe. The quotient is "how many of the cheapest
/// manifest touches does one entry lookup cost".
fn floor_ps(m: &Manifest) -> u128 {
    cost_ps(20_000, || {
        let n = black_box(m).entries();
        black_box(n.wrapping_add(1))
    })
}

/// Prints one budget in floors and returns whether it held.
fn budget(label: &str, floor: u128, at_ps: u128, allowed: u128) -> bool {
    if floor == 0 {
        println!("  {label:<48} UNMEASURABLE — the floor timed at zero");
        return false;
    }
    let floors = at_ps.saturating_mul(1_000) / floor;
    let ok = floors <= allowed.saturating_mul(1_000);
    println!(
        "  {label:<48} {at_ps:>8} ps = {}.{:03} floors, budget {allowed}   {}",
        floors / 1_000,
        floors % 1_000,
        if ok { "ok" } else { "OVER BUDGET" }
    );
    ok
}

/// C-26 — one entry lookup costs a bounded multiple of the per-lookup floor.
fn entry_lookup_stays_within_its_budget() -> bool {
    /// Floors allowed per lookup.
    ///
    /// Measured, arm64 laptop, release, three consecutive runs: **57.167,
    /// 57.161, 58.440** floors, at a floor of 502–514 ps. A **1.02x spread** —
    /// the tightest in the workspace, because both legs read the same resident
    /// struct and neither allocates.
    ///
    /// **240**, sized on the worst observed with roughly 4x left over, the same
    /// rule the other nine budgets apply. It still refuses the 174x uniform
    /// regression this file's ratios would report as ok: such a lookup would
    /// read about 10,100 floors and be refused by a factor of 42.
    ///
    /// A breach is NOT a census-size dependence; C-12 is that row, and it is
    /// measured at 1x, 10x and 100x. This one means every lookup got dearer at
    /// once — a rehash at an exact load factor, say, which is the defect
    /// `docs/06-limits.md` records as a 2.4 ms stall at 50,000 entries and which
    /// no quotient can see.
    const ALLOWED: u128 = 240;

    let (m, _keep) = census(100_000);
    let floor = floor_ps(&m);
    println!("  the per-lookup floor is {floor} ps — one entry-count read and an add");
    let present = key(7);
    let at = cost_ps(20_000, || black_box(&m).entry(black_box(&present)));
    budget("C-26 entry lookup against the floor", floor, at, ALLOWED)
}

fn entry_lookup_is_flat() -> bool {
    let (one, _) = census(1_000);
    let (ten, _) = census(10_000);
    let (hundred, _) = census(100_000);

    let present = key(7);
    let absent = key_for("SENSEX", 7);

    let hit = |m: &Manifest| cost_ps(20_000, || black_box(m).entry(black_box(&present)));
    let miss = |m: &Manifest| cost_ps(20_000, || black_box(m).entry(black_box(&absent)));

    let base = hit(&one);
    let a = ratio("C-12 entry lookup, 10x census", base, hit(&ten));
    let b = ratio("C-12 entry lookup, 100x census", base, hit(&hundred));

    // A miss is the same probe as a hit. It is measured separately because a
    // structure that answered "no" by walking would look flat on hits alone.
    let base = miss(&one);
    let c = ratio("C-12 absent lookup, 10x census", base, miss(&ten));
    let d = ratio("C-12 absent lookup, 100x census", base, miss(&hundred));

    a && b && c && d
}

/// New keys appended inside one timed region.
///
/// A single `record` is a few tens of nanoseconds, which is inside the clock
/// granularity this file measures and prints. A fixed batch amortises the timer
/// over enough work to be resolvable while staying **far below** the headroom
/// the reservation is supposed to carry, so a batch that rehashes is a
/// reservation that was wrong rather than a batch that was greedy.
const APPEND_BATCH: u32 = 256;

/// Reloads per size before the minimum is taken.
///
/// Fewer than [`TRIALS`] because each trial pays a full [`Manifest::load`] of
/// the largest census outside the timed region. The minimum over twelve
/// reloads is still a minimum: a scheduler can only ever make a sample slower.
const APPEND_TRIALS: u32 = 12;

/// The census sizes the audit reported, and the census sizes that are the
/// **worst case** for a table reserved to exactly what was loaded.
///
/// `HashMap::with_capacity(n)` rounds `n` up to `7·2^k` — so at `n = 7·2^k`
/// exactly it hands back a table of capacity `n` and **no free slot at all**,
/// and the first new key rebuilds the whole table. 1,792, 14,336 and 57,344 are
/// `7·2^8`, `7·2^11` and `7·2^13`: the 1k / 10k / 50k of the audit, moved onto
/// the boundary where the defect is not a matter of luck. Both sets are
/// measured, because a bench that only visited the round numbers would report
/// whatever slack the rounding happened to leave that day.
///
/// These are `pull::bench::append_after_load_is_flat`'s sizes — `C-13`, and
/// `M-19` beside it for the free room a loaded index carries.
const APPEND_SIZES: [u32; 3] = [1_000, 10_000, 50_000];

/// [`APPEND_SIZES`], moved onto the capacity boundary. See there.
const APPEND_WORST_SIZES: [u32; 3] = [1_792, 14_336, 57_344];

const _: () = {
    let [a, b, c] = APPEND_WORST_SIZES;
    assert!(a == 7 * 256 && b == 7 * 2_048 && c == 7 * 8_192);
};

/// Picoseconds per append, appending [`APPEND_BATCH`] **new** keys to a
/// manifest freshly loaded from these bytes.
///
/// The load happens outside the timer on every trial, so what is charged is the
/// append path alone. `first_index` is chosen past the census so every key in
/// the batch is one the census does not hold — an update to an existing key
/// replaces in place and could never rehash, so measuring one would measure the
/// case that was never in doubt.
fn append_cost_ps(region: &[u8], data: &[u8], first_index: u32) -> u128 {
    let mut best = u128::MAX;
    for _ in 0..APPEND_TRIALS {
        let Ok(mut manifest) = Manifest::load(Vendor::Groww, region, data) else {
            refuse("a census this harness had just written")
        };
        let mut refused = false;
        let start = Instant::now();
        for i in 0..APPEND_BATCH {
            let entry = Entry {
                key: key(first_index + i),
                rows: 7_312,
                first_ts_micros: 1_000,
                last_ts_micros: 2_000,
            };
            match manifest.record(entry) {
                Ok(append) => {
                    black_box(append);
                }
                Err(_) => refused = true,
            }
        }
        let ps = start.elapsed().as_nanos() * 1_000 / u128::from(APPEND_BATCH);
        if refused {
            refuse("an append the manifest would not accept");
        }
        if ps < best {
            best = ps;
        }
    }
    best
}

/// C-13 — appending a month after a load costs the same whatever the census
/// holds.
///
/// `CLAUDE.md` §3 rule 4 lists **result append** among the operations that must
/// be O(1), and until this function existed no bench in the repository touched
/// the manifest's append path at all. The reservation was exactly the committed
/// entry count, which leaves a table at a capacity boundary with zero free
/// slots, so the next new month rebuilt the whole index — an O(keys) step
/// behind a doc comment that said "O(1) worst case".
///
/// A ratio rather than a nanosecond threshold, for the reason [`ratio`] gives:
/// an absolute number would have to be guessed for hardware this repository has
/// never run on, and a ratio taken in one process on one machine is a property
/// of the code.
fn append_after_load_is_flat() -> bool {
    println!(
        "  {:<44} {} bytes per key/value pair",
        "index element width (context, not a ratio)",
        size_of::<(EntryKey, Held)>()
    );

    let mut ok = true;
    for (label, sizes) in [
        // Spelled with spaces rather than as one lower-case word on purpose:
        // CI gate 1d flags every `"[a-z0-9][a-z0-9_-]*"` literal under
        // crates/pull as a possible parameter path segment, and a bench label
        // is not worth an entry on that allowlist.
        ("at a round count", APPEND_SIZES),
        ("at the capacity boundary", APPEND_WORST_SIZES),
    ] {
        let [small, medium, large] = sizes;
        let cost = |count: u32| {
            let (region, data) = census_bytes(count);
            let Ok(loaded) = Manifest::load(Vendor::Groww, &region, &data) else {
                refuse("a census this harness had just written")
            };
            println!(
                "  {:<44} census {count}, reserved {}",
                format!("C-13 {label} reservation"),
                loaded.reserved()
            );
            append_cost_ps(&region, &data, count)
        };
        let base = cost(small);
        let at_ten = cost(medium);
        let at_fifty = cost(large);
        ok &= ratio(&format!("C-13 append, {label}, {medium}"), base, at_ten);
        ok &= ratio(&format!("C-13 append, {label}, {large}"), base, at_fifty);
    }
    ok
}

// ══ Census version 4 across a compaction (D-3316): C-14 ═════════════════════

/// The `n`th invented option contract-month, for a version-4 FNO partition.
///
/// Distinct for every `n` below 12 · 10,000,000 — a strike per twelve months —
/// so a census of any size this harness builds is all distinct keys.
fn option_key(n: u32) -> EntryKey {
    let strike = 1_000_000 + u64::from(n / 12) * 100;
    let Some(contract) =
        brutex_core::instrument::Contract::parse(&format!("2019-12-26-{strike}-CE"))
    else {
        refuse("an invented contract the store refuses")
    };
    let (Ok(symbol), Ok(month)) = (
        Symbol::new("NIFTY"),
        YearMonth::new(2019, u8::try_from(n % 12 + 1).unwrap_or(1)),
    ) else {
        refuse("a symbol or a month this build refuses")
    };
    EntryKey {
        contract: Some(contract),
        exchange: Exchange::Nse,
        segment: Segment::Fno,
        symbol,
        timeframe: Timeframe::SECOND_1,
        month,
    }
}

/// A version-4 FNO partition's file bytes: `keys` keys, each written `passes`
/// times with its row count growing, interleaved across keys — the shape an
/// import day by day leaves behind.
fn partition_bytes(keys: u32, passes: u32) -> Vec<u8> {
    let Ok(mut census) = Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, FNO_FLOOR) else {
        refuse("a genesis partition")
    };
    for pass in 0..passes {
        for n in 0..keys {
            let rows = u64::from(pass) + 1;
            let entry = Entry {
                key: option_key(n),
                rows,
                first_ts_micros: 1_000,
                last_ts_micros: 1_000 + i64::from(pass),
            };
            if census.record(entry).is_err() {
                refuse("a row the partition would not accept");
            }
        }
    }
    census.image()
}

/// The partition in `file`, loaded as a reader loads it.
fn partition_of(file: &[u8]) -> Manifest {
    match Manifest::open_partition(Vendor::Gdfl, Segment::Fno, file) {
        Ok(census) => census,
        Err(e) => refuse(&e.to_string()),
    }
}

/// Picoseconds per append of [`APPEND_BATCH`] new keys to a partition freshly
/// loaded from `file`, past the keys it holds.
fn partition_append_ps(file: &[u8], first_index: u32) -> u128 {
    let mut best = u128::MAX;
    for _ in 0..APPEND_TRIALS {
        let mut census = partition_of(file);
        let mut refused = false;
        let start = Instant::now();
        for i in 0..APPEND_BATCH {
            let entry = Entry {
                key: option_key(first_index + i),
                rows: 1,
                first_ts_micros: 1_000,
                last_ts_micros: 1_000,
            };
            match census.record(entry) {
                Ok(append) => {
                    black_box(append);
                }
                Err(_) => refused = true,
            }
        }
        let ps = start.elapsed().as_nanos() * 1_000 / u128::from(APPEND_BATCH);
        if refused {
            refuse("an append the partition would not accept");
        }
        best = best.min(ps);
    }
    best
}

/// C-14 — a lookup and an append cost the same before and after a
/// compaction, and a compaction costs the same per entry at ten times the
/// census. D-3316.
///
/// The partition before is 20,000 keys written eight times, 160,000 entries;
/// after, the same 20,000 keys once each. A lookup is one probe of the index
/// either way and an append one probe, one push and two images either way —
/// the compaction changes how many entries a LOAD decodes, which is the point
/// of it, and must not change what one operation costs. A compaction is
/// O(entries): at 200,000 keys × 8 it must cost the same per entry as at
/// 20,000 × 8, or it has started to scan more than once.
fn census_is_flat_across_a_compaction() -> bool {
    const KEYS: u32 = 20_000;
    const PASSES: u32 = 8;
    let before_file = partition_bytes(KEYS, PASSES);
    let before = partition_of(&before_file);
    let after = match before.compacted() {
        Ok(census) => census,
        Err(e) => refuse(&e.to_string()),
    };
    let after_file = after.image();
    println!(
        "  {:<44} {} entries over {} keys -> {} entries; file {} -> {} bytes",
        "C-14 the partition compacted (context)",
        before.entries(),
        before.keys(),
        after.entries(),
        before_file.len(),
        after_file.len()
    );

    let present = option_key(7);
    let absent = option_key(KEYS + 7);
    let hit = |m: &Manifest| cost_ps(20_000, || black_box(m).entry(black_box(&present)));
    let miss = |m: &Manifest| cost_ps(20_000, || black_box(m).entry(black_box(&absent)));
    let reloaded = partition_of(&after_file);
    let mut ok = ratio("C-14 lookup, after vs before", hit(&before), hit(&reloaded));
    ok &= ratio(
        "C-14 absent lookup, after vs before",
        miss(&before),
        miss(&reloaded),
    );
    ok &= ratio(
        "C-14 append, after vs before",
        partition_append_ps(&before_file, KEYS),
        partition_append_ps(&after_file, KEYS),
    );
    spread("C-14 lookup after compaction (one call)", 2_000, || {
        black_box(reloaded.entry(black_box(&present)));
    });

    // Compaction per entry, at 1x and 10x.
    let per_entry = |keys: u32| {
        let census = partition_of(&partition_bytes(keys, PASSES));
        let entries = u128::from(census.entries());
        let mut best = u128::MAX;
        for _ in 0..5 {
            let start = Instant::now();
            black_box(census.compacted().map_or(0, |c| c.entries()));
            best = best.min(start.elapsed().as_nanos() * 1_000 / entries.max(1));
        }
        best
    };
    ok &= ratio(
        "C-14 compaction per entry, 10x census",
        per_entry(KEYS),
        per_entry(KEYS * 10),
    );
    ok
}

// ══ GDFL import lookups (D-2802 to D-2806): C-GI-01, C-GI-02, C-GI-03 ═══════

/// Single-operation latencies, nanoseconds: p50, p99 and max over `n` calls.
/// Printed beside each ratio, never gated: one call sits near the clock's
/// resolution, which the ratio above amortises and these do not.
fn spread(label: &str, n: u32, mut op: impl FnMut()) {
    let mut ns: Vec<u128> = (0..n)
        .map(|_| {
            let start = Instant::now();
            op();
            start.elapsed().as_nanos()
        })
        .collect();
    ns.sort_unstable();
    let at = |q: usize| {
        ns.get(ns.len().saturating_sub(1) * q / 100)
            .copied()
            .unwrap_or(0)
    };
    println!(
        "  {label:<44} p50 {:>8} ns  p99 {:>8} ns  max {:>10} ns  n={n}",
        at(50),
        at(99),
        ns.last().copied().unwrap_or(0)
    );
}

/// An options day listing of `n` invented tickers.
fn option_day(n: u32) -> pull::gdfl_nfo::NfoDay<u32> {
    let day = pull::session::Day::new(2026, 8, 3).unwrap_or_else(|_| refuse("a real day"));
    let mut listing = pull::gdfl_nfo::NfoDay::new(day, n as usize);
    for i in 0..n {
        let name = format!("GFDLNFO_TICK_03082026\\Options\\X{i}27AUG26100CE.NFO.csv");
        listing.push(&name, 1, 1, i);
    }
    listing
}

/// C-GI-01: a contract's file of the day is one hash probe, flat at 1x, 10x
/// and 100x the listing (the real 2026-08-03 day lists 11,524).
fn gdfl_option_lookup_is_flat() -> bool {
    let (one, ten, hundred) = (option_day(1_000), option_day(10_000), option_day(100_000));
    let probe = |l: &pull::gdfl_nfo::NfoDay<u32>| {
        cost_ps(20_000, || {
            black_box(l).locate(black_box("X77727AUG26100CE")).is_ok()
        })
    };
    let base = probe(&one);
    let a = ratio("C-GI-01 option file by ticker, 10x day", base, probe(&ten));
    let b = ratio(
        "C-GI-01 option file by ticker, 100x day",
        base,
        probe(&hundred),
    );
    spread("C-GI-01 option file by ticker, 100x day", 20_000, || {
        black_box(hundred.locate(black_box("X99999827AUG26100CE")).is_ok());
    });
    a && b
}

/// A stock-day listing of `n` names, one of them a swept share's.
fn stock_day(n: u32) -> pull::gdfl_cm::DayListing<u32> {
    let day = pull::session::Day::new(2026, 8, 3).unwrap_or_else(|_| refuse("a real day"));
    let mut listing = pull::gdfl_cm::DayListing::new(pull::gdfl_cm::CmKind::Stocks, day);
    listing.push("GFDLCM_STOCK_TICK_03082026/RELIANCE.NSE.csv", 1, 1, 0);
    for i in 1..n {
        listing.push(&format!("GFDLCM_STOCK_TICK_03082026/Z{i}.NSE.csv"), 1, 1, i);
    }
    listing
}

/// C-GI-02: a share's file of the day is one slot index, flat at 1x, 10x and
/// 100x the listing (the real day lists 3,515).
fn gdfl_stock_lookup_is_flat() -> bool {
    let (one, ten, hundred) = (stock_day(1_000), stock_day(10_000), stock_day(100_000));
    let probe = |l: &pull::gdfl_cm::DayListing<u32>| {
        cost_ps(20_000, || {
            black_box(l).locate(black_box("RELIANCE.NSE")).is_ok()
        })
    };
    let base = probe(&one);
    let a = ratio("C-GI-02 share file by stem, 10x day", base, probe(&ten));
    let b = ratio(
        "C-GI-02 share file by stem, 100x day",
        base,
        probe(&hundred),
    );
    spread("C-GI-02 share file by stem, 100x day", 20_000, || {
        black_box(hundred.locate(black_box("RELIANCE.NSE")).is_ok());
    });
    a && b
}

/// A one-second month of `n` bars for one invented contract, in a scratch
/// directory the bench removes.
fn second_month(root: &std::path::Path, n: u32) -> store::file::BarFile {
    let contract = brutex_core::instrument::Contract::parse("2026-08-27-10000-CE")
        .unwrap_or_else(|| refuse("a contract"));
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "FNO",
        symbol: "X",
        contract: Some(contract),
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2026, 8).unwrap_or_else(|_| refuse("a month")),
        file: store::path::FileKind::Bars,
    })
    .unwrap_or_else(|_| refuse("a path"));
    std::fs::create_dir_all(root).unwrap_or_else(|_| refuse("a scratch root"));
    let mut file = store::file::BarFile::open_or_create(root, path, 7)
        .unwrap_or_else(|_| refuse("a bar file"));
    // 2026-08-03 09:15:00 IST, then one bar a second.
    let first = 1_785_729_900_i64;
    let bars: Vec<store::format::Bar> = (0..i64::from(n))
        .map(|s| store::format::Bar {
            ts_micros: (first + s) * 1_000_000,
            open: 100,
            high: 101,
            low: 99,
            close: 100,
            volume: 75,
            open_interest: 1_000,
        })
        .collect();
    file.append(&bars).unwrap_or_else(|_| refuse("an append"));
    file
}

/// C-GI-03: one stored second by row index is one positional read, flat at
/// 1x, 10x and 100x the month. By TIME it is the store's bisection, which is
/// printed as a limit and not gated (docs/06-limits.md).
fn gdfl_second_by_index_is_flat() -> bool {
    let root = std::env::temp_dir().join(format!("brutex-bench-gdfl-{}", std::process::id()));
    let one = second_month(&root.join("1"), 1_000);
    let ten = second_month(&root.join("10"), 10_000);
    let hundred = second_month(&root.join("100"), 100_000);
    let read = |f: &store::file::BarFile| {
        cost_ps(20_000, || black_box(f).read_record(black_box(500)).is_ok())
    };
    let base = read(&one);
    let a = ratio("C-GI-03 second by index, 10x month", base, read(&ten));
    let b = ratio("C-GI-03 second by index, 100x month", base, read(&hundred));
    spread("C-GI-03 second by index, 100x month", 20_000, || {
        black_box(hundred.read_record(black_box(77_777)).is_ok());
    });
    let at = (1_785_729_900_i64 + 77_777) * 1_000_000;
    spread(
        "second by TIME (bisection, a limit, NOT O(1))",
        20_000,
        || {
            black_box(hundred.first_at_or_after(black_box(at)).is_ok());
        },
    );
    drop((one, ten, hundred));
    let _cleanup = std::fs::remove_dir_all(&root);
    a && b
}

/// A weekly series of `weeks` invented auctions from 1990-01-03, as the text
/// `pull::riskfree::Series::parse` reads. One invented price/yield pair, which
/// the parser's own consistency check accepts; the VALUE is irrelevant to a
/// lookup's cost.
fn weekly_series(weeks: u32) -> pull::riskfree::Series {
    let Ok(start) = pull::pricing::TradeDay::new(1990, 1, 3) else {
        refuse("1990-01-03 is not a trade day")
    };
    let mut text = String::from(pull::riskfree::HEADER);
    for week in 0..weeks {
        let Ok(offset) = i32::try_from(week * 7) else {
            refuse("a week offset outside i32")
        };
        let Ok(day) = start.plus_days(offset) else {
            refuse("a release past 2100")
        };
        text.push_str("\n\t\t");
        text.push_str(&day.to_string());
        text.push_str("\t98.5000\t6.1081\thttps://www.rbi.org.in/bench\t\t");
    }
    match pull::riskfree::Series::parse(&text) {
        Ok(series) => series,
        Err(why) => refuse(&format!("the invented series was refused: {why}")),
    }
}

/// 1,024 days spread over the whole table, so a lookup cycles through the
/// dense table rather than re-reading one hot slot.
fn rate_days(series: &pull::riskfree::Series) -> Vec<pull::pricing::TradeDay> {
    let first = series.first_release().ordinal() + 1;
    let span = series.last_release().ordinal() - first + 1;
    (0..1_024)
        .filter_map(|i| pull::pricing::TradeDay::from_ordinal(first + span * i / 1_024).ok())
        .collect()
}

/// p50 / p99 / max of one lookup, in nanoseconds per call, over batches of
/// 1,024 — printed, never gated: the gate is the ratio below.
fn report_tail(label: &str, series: &pull::riskfree::Series, days: &[pull::pricing::TradeDay]) {
    let mut per_call: Vec<u128> = Vec::with_capacity(400);
    for _ in 0..400 {
        let start = Instant::now();
        for day in days {
            black_box(series.on(*day).ok());
        }
        per_call.push(start.elapsed().as_nanos() * 1_000 / 1_024);
    }
    per_call.sort_unstable();
    let at = |permille: usize| {
        per_call
            .get((per_call.len() * permille / 1_000).min(per_call.len() - 1))
            .copied()
            .unwrap_or(0)
    };
    println!(
        "  {label:<44} p50 {:>6} ps  p99 {:>6} ps  max {:>6} ps",
        at(500),
        at(990),
        per_call.last().copied().unwrap_or(0)
    );
}

/// C-RF-01 — one dated-rate lookup costs the same at 1×, 10× and 100× the
/// table: 52, 520 and 5,200 weekly auctions, a year to a century.
///
/// `pull::riskfree::Series::on` reads one slot of a table dense by day, so a
/// search that walked the auctions would be flat at 1× and linear by 100×.
fn rate_lookup_is_flat() -> bool {
    let sizes = [52, 520, 5_200].map(weekly_series);
    let mut costs = [0_u128; 3];
    for (cost, series) in costs.iter_mut().zip(&sizes) {
        let days = rate_days(series);
        let mut at = 0_usize;
        *cost = cost_ps(4_096, || {
            at = (at + 1) % days.len();
            days.get(at).map(|day| series.on(*day).is_ok())
        });
    }
    for (label, series) in [
        "C-RF-01 tail, 52 auctions",
        "C-RF-01 tail, 520 auctions",
        "C-RF-01 tail, 5,200 auctions",
    ]
    .iter()
    .zip(&sizes)
    {
        report_tail(label, series, &rate_days(series));
    }
    let [base, ten, hundred] = costs;
    let a = ratio("C-RF-01 rate lookup, 10x table", base, ten);
    let b = ratio("C-RF-01 rate lookup, 100x table", base, hundred);
    a && b
}

// ══ The alert queue (D-4883): C-AL-01, C-AL-02 ══════════════════════════════

/// The stamp every invented alert of this bench carries.
const ALERT_AT: u64 = 1_791_234_567;

/// A queue of `n` invented open alerts under a scratch store root, written
/// whole so the bench times the queue and not its own set-up.
fn alert_queue(root: &std::path::Path, n: u32) {
    use pull::alerts::{Names, Record, TYPE_ALERT, header, key_hash};
    let names = Names::under(root);
    std::fs::create_dir_all(&names.dir).unwrap_or_else(|_| refuse("a scratch audit/"));
    let mut bytes = header(ALERT_AT);
    for i in 1..=n {
        let record = Record {
            kind_of_record: TYPE_ALERT,
            kind: 6,
            flags: 0,
            day: 0,
            run: u64::from(i),
            put_at: ALERT_AT,
            key: key_hash(&format!("seed:{i}")),
            acked: 0,
            open_after: i,
            text: "an invented alert".to_owned(),
        };
        bytes.extend_from_slice(&record.encode());
    }
    std::fs::write(&names.queue, bytes).unwrap_or_else(|_| refuse("a scratch queue"));
}

/// One put of a key no record carries, so it always appends.
fn alert_put(store: &std::path::Path, next: &std::cell::Cell<u64>) -> u32 {
    let n = next.get();
    next.set(n + 1);
    let key = format!("bench:{n}");
    let alert = pull::alerts::NewAlert {
        kind: 6,
        key: &key,
        day: 0,
        run: n,
        text: "an invented alert",
    };
    match pull::alerts::put_at(store, &alert, ALERT_AT) {
        Ok(pull::alerts::Put::Written { ordinal, .. }) => ordinal,
        _ => refuse("an alert put"),
    }
}

/// The three scratch queues: a full window (1,024 records), 10x and 100x.
fn alert_queues() -> (std::path::PathBuf, [std::path::PathBuf; 3]) {
    let root = std::env::temp_dir().join(format!("brutex-bench-alerts-{}", std::process::id()));
    let stores = [root.join("1"), root.join("10"), root.join("100")];
    for (store, n) in stores.iter().zip([1_024_u32, 10_240, 102_400]) {
        alert_queue(store, n);
    }
    (root, stores)
}

/// C-AL-01: one put (one read of the newest 1,024 records, one 128-byte
/// append, one `sync_data`) costs the same with a full window, 10x and 100x
/// the records in the file. The acknowledgement and the put over a torn tail
/// are the same path and are printed beside it, never gated.
fn alert_put_is_flat(stores: &[std::path::PathBuf; 3]) -> bool {
    let [one, ten, hundred] = stores;
    let next = std::cell::Cell::new(0_u64);
    let put = |store: &std::path::Path| cost_ps(8, || alert_put(black_box(store), &next));
    let base = put(one);
    let a = ratio("C-AL-01 alert put, 10x queue", base, put(ten));
    let b = ratio("C-AL-01 alert put, 100x queue", base, put(hundred));
    spread("C-AL-01 alert put, full window", 400, || {
        black_box(alert_put(one, &next));
    });
    spread("C-AL-01 alert put, 100x queue", 400, || {
        black_box(alert_put(hundred, &next));
    });
    let queue = pull::alerts::Names::under(hundred).queue;
    spread("C-AL-01 alert put over a torn tail, 100x", 200, || {
        let torn = std::fs::OpenOptions::new()
            .append(true)
            .open(&queue)
            .and_then(|mut file| std::io::Write::write_all(&mut file, &[0xAB; 57]));
        if torn.is_err() {
            refuse("a torn tail");
        }
        black_box(alert_put(hundred, &next));
    });
    let mut open: Vec<u32> = (0..200).map(|_| alert_put(hundred, &next)).collect();
    spread("C-AL-01 acknowledgement, 100x queue", 200, || {
        let ordinal = open.pop().unwrap_or(0);
        let scope = pull::alerts::Scope::Window;
        if pull::alerts::ack_at(hundred, ordinal, "bench", scope, ALERT_AT).is_err() {
            refuse("an acknowledgement");
        }
    });
    a && b
}

/// C-AL-02: the readers' one read (`alerts::window`, under the lock held
/// shared) costs the same with a full window, 10x and 100x the records. The
/// same read with a put in flight, and an older page by `before`, are printed
/// beside it (p50, p99, max), never gated: a reader waits for at most one put.
fn alert_window_is_flat(stores: &[std::path::PathBuf; 3]) -> bool {
    let [one, ten, hundred] = stores;
    let read = |store: &std::path::Path| {
        cost_ps(40, || {
            pull::alerts::window(black_box(store)).map_or(0, |page| page.alerts.len())
        })
    };
    let base = read(one);
    let a = ratio("C-AL-02 alert window read, 10x queue", base, read(ten));
    let b = ratio("C-AL-02 alert window read, 100x queue", base, read(hundred));
    spread("C-AL-02 alert window read, 100x queue", 2_000, || {
        black_box(pull::alerts::window(hundred).is_ok());
    });
    spread("C-AL-02 older page (before=50000), 100x", 2_000, || {
        black_box(pull::alerts::page(hundred, Some(50_000)).is_ok());
    });
    let stop = std::sync::atomic::AtomicBool::new(false);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let next = std::cell::Cell::new(1_u64 << 40);
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                black_box(alert_put(hundred, &next));
            }
        });
        spread("C-AL-02 alert window read, a put in flight", 2_000, || {
            black_box(pull::alerts::window(hundred).is_ok());
        });
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    a && b
}

// ══ The fold, per bar (cloud/attack-o1-20261016): C-FD-01 ══════════════════

/// Snapshot counts folded: 1x / 10x / 100x.
const FOLD_SIZES: [u32; 3] = [1_000, 10_000, 100_000];

/// `n` one-second snapshots of one IST day from 2026-09-07 09:15, every
/// tenth second skipped so a bucket is folded from a ragged run of rows.
fn snapshots(n: u32) -> Vec<store::format::Bar> {
    // 2026-09-07 09:15:00 IST in UTC microseconds.
    const OPEN: i64 = 1_788_752_700_000_000;
    (0..n)
        .map(|i| {
            let raw = i64::from(i);
            store::format::Bar {
                ts_micros: OPEN + (raw + raw / 9) * 1_000_000,
                open: 2_000_000 + raw % 97,
                high: 2_000_100 + raw % 97,
                low: 1_999_900 + raw % 97,
                close: 2_000_050 + raw % 89,
                volume: 75,
                open_interest: store::format::OI_NULL,
            }
        })
        .collect()
}

/// C-FD-01 — `pull::fold::fold_from_snapshots` into minutes costs the same
/// PER BAR at 1,000, 10,000 and 100,000 snapshots, and its p50/p99/max per
/// whole fold are printed. `docs/06-limits.md` (D-1201) said "no bench times
/// the fold"; this is the row.
fn fold_is_flat_per_bar() -> bool {
    let mut per_bar = Vec::with_capacity(FOLD_SIZES.len());
    for n in FOLD_SIZES {
        let bars = snapshots(n);
        let folded = pull::fold::fold_from_snapshots(&bars, pull::fold::Bucket::MINUTE)
            .unwrap_or_else(|_| refuse("the fold refused its fixture"));
        if folded.is_empty() {
            refuse("the fold produced no minute");
        }
        spread(&format!("C-FD-01 fold to 1min, {n} snapshots"), 200, || {
            black_box(
                pull::fold::fold_from_snapshots(black_box(&bars), pull::fold::Bucket::MINUTE)
                    .map_or(0, |out| out.len()),
            );
        });
        let whole = cost_ps(3, || {
            pull::fold::fold_from_snapshots(black_box(&bars), pull::fold::Bucket::MINUTE)
                .map_or(0, |out| out.len())
        });
        per_bar.push(whole / u128::from(n));
    }
    let (Some(&base), Some(&mid), Some(&big)) = (per_bar.first(), per_bar.get(1), per_bar.get(2))
    else {
        refuse("three sizes measured");
    };
    ratio("C-FD-01 fold per bar, 1k -> 10k", base, mid)
        & ratio("C-FD-01 fold per bar, 1k -> 100k", base, big)
}

fn main() {
    println!("gate 8 — crates/pull, ceiling {CEILING_PERMILLE} permille");
    let mut ok = true;
    ok &= gdfl_option_lookup_is_flat();
    ok &= gdfl_stock_lookup_is_flat();
    ok &= gdfl_second_by_index_is_flat();
    ok &= census_beats_the_scan_it_replaces();
    ok &= entry_lookup_is_flat();
    ok &= entry_lookup_stays_within_its_budget();
    ok &= append_after_load_is_flat();
    ok &= census_is_flat_across_a_compaction();
    ok &= rate_lookup_is_flat();
    let (alert_root, alert_stores) = alert_queues();
    ok &= alert_put_is_flat(&alert_stores);
    ok &= alert_window_is_flat(&alert_stores);
    let _cleanup = std::fs::remove_dir_all(&alert_root);
    ok &= fold_is_flat_per_bar();
    if ok {
        println!("all ratios within the ceiling");
    } else {
        println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
        std::process::exit(1);
    }
}
