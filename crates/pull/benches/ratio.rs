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
//! write side. They are functions in **this file**, which is why the layer is
//! asserted rather than described.
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

use pull::manifest::{Commit, ENTRY_LEN, Entry, EntryKey, HEADER_LEN, Held, Manifest};

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

/// One July-2025 session of one-minute rows, as `crates/pull/tests/derive.rs`
/// builds them: `day` is the day of the month.
fn session_member(day: u8) -> pull::archive::Member {
    // 2025-07-01 09:15:00 IST as a UTC epoch second.
    const OPEN_UTC: i64 = 1_751_341_500;
    let open = OPEN_UTC + (i64::from(day) - 1) * 86_400;
    let rows = (0..375_i64)
        .map(|m| {
            let base = 2_550_000 + ((m * 37) % 211 - 105) * 25;
            pull::fetch::RawRow {
                timestamp: open + m * 60,
                open: base,
                high: base + 40,
                low: base - 35,
                close: base + 10,
                volume: 400 + m,
                open_interest: Some(500_000 + m * 3),
            }
        })
        .collect();
    pull::archive::Member {
        path: std::path::PathBuf::from("/bought/NIFTY.csv"),
        instrument: "NIFTY".to_owned(),
        rows,
    }
}

/// The July-2025 days the canonical calendar holds as full sessions.
fn full_sessions() -> Vec<u8> {
    (1..=31_u8)
        .filter(|&day| {
            pull::session::Day::new(2025, 7, day).is_some_and(|date| {
                matches!(
                    pull::calendar::kind_of(i64::from(date.days_from_epoch())),
                    pull::calendar::DayKind::Open(session) if session.bars() == 375
                )
            })
        })
        .collect()
}

/// The `s`-th order statistic of `samples` at `permille` (p50 = 500).
fn quantile(samples: &mut [u128], permille: usize) -> u128 {
    samples.sort_unstable();
    let at = (samples.len().saturating_sub(1) * permille) / 1_000;
    samples.get(at).copied().unwrap_or(0)
}

/// ET-bars-candles-store-1, -8 and rederive, D-2240: **MEASURED**.
///
/// `derive_all` re-reads and re-folds the whole month on every batch, so the
/// cost of ingesting the s-th session of a month grows with s. This fills a
/// fresh store's July 2025 one session per `from_members` call, as an
/// operator filling a month day by day does, `ROUNDS` times, and prints
/// p50, p99 and max of one call's wall time at the first, middle and last
/// session. The bound it asserts is the one `docs/06-limits.md` states: the
/// per-call cost grows at most linearly in s, so the last call costs no more
/// than `CEILING_PERMILLE` thousandths of s times the first (a call that
/// re-derived more than the month would breach it). It does not assert that
/// the growth is absent: that would be the incremental fold the store format
/// cannot resume (D-0955).
fn a_month_filled_session_by_session_rederives_linearly() -> bool {
    const ROUNDS: usize = 9;
    let days = full_sessions();
    let sessions = days.len();
    let mut per_session: Vec<Vec<u128>> = vec![Vec::with_capacity(ROUNDS); sessions];
    for round in 0..ROUNDS {
        let root = std::env::temp_dir().join(format!(
            "brutex-pull-bench-rederive-{}-{round}",
            std::process::id()
        ));
        let _fresh = std::fs::remove_dir_all(&root);
        if std::fs::create_dir_all(&root).is_err() {
            println!("BREACH rederive: no scratch root");
            return false;
        }
        for (at, &day) in days.iter().enumerate() {
            let Some(date) = pull::session::Day::new(2025, 7, day) else {
                return false;
            };
            let Ok(window) = pull::session::Window::new(date, date) else {
                return false;
            };
            let request = pull::fetch::BarRequest {
                instrument_id: String::new(),
                listing: pull::vendor::Listing::Index,
                window,
                granularity: pull::vendor::Granularity::Minute1,
            };
            let plan = pull::ingest::Plan {
                calendar: pull::calendar::Runtime::default(),
                cash_schedule: None,
                columns: pull::csv::Columns::TrueDataIndex,
                request: &request,
                encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
                scale: pull::vendor::PriceScale::Paisa,
                vendor: Vendor::TrueData,
                exchange: "NSE",
                segment: "INDEX",
                contract: None,
            };
            let member = session_member(day);
            let start = Instant::now();
            let done = black_box(pull::ingest::from_members(
                std::slice::from_ref(&member),
                &root,
                plan,
            ));
            let took = start.elapsed().as_nanos();
            if !done.failures.is_empty() {
                println!("BREACH rederive: session {day} refused: {:?}", done.failures);
                return false;
            }
            if let Some(samples) = per_session.get_mut(at) {
                samples.push(took);
            }
        }
        let _cleanup = std::fs::remove_dir_all(&root);
    }
    let mut report = |at: usize| {
        let samples = per_session.get_mut(at).map_or(&mut [][..], Vec::as_mut_slice);
        let (p50, p99, max) = (
            quantile(samples, 500),
            quantile(samples, 990),
            quantile(samples, 1_000),
        );
        println!(
            "rederive: session {:>2} of {sessions}: p50 {:>9} ns, p99 {:>9} ns, max {:>9} ns",
            at + 1,
            p50,
            p99,
            max
        );
        p50
    };
    let first = report(0);
    let _middle = report(sessions / 2);
    let last = report(sessions.saturating_sub(1));
    let envelope = first
        .saturating_mul(u128::try_from(sessions).unwrap_or(u128::MAX))
        .saturating_mul(CEILING_PERMILLE)
        / 1_000;
    let ok = sessions > 1 && last <= envelope;
    println!(
        "rederive: last p50 {last} ns against a linear envelope of {envelope} ns ({})",
        if ok { "within" } else { "BREACH" }
    );
    ok
}

fn main() {
    println!("gate 8 — crates/pull, ceiling {CEILING_PERMILLE} permille");
    let mut ok = true;
    ok &= census_beats_the_scan_it_replaces();
    ok &= entry_lookup_is_flat();
    ok &= entry_lookup_stays_within_its_budget();
    ok &= append_after_load_is_flat();
    ok &= a_month_filled_session_by_session_rederives_linearly();
    if ok {
        println!("all ratios within the ceiling");
    } else {
        println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
        std::process::exit(1);
    }
}
