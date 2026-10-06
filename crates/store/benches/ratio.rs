//! Gate 8 for `crates/store` — per-operation cost stays constant as the input
//! it sits in grows.
//!
//! # Why this file exists
//!
//! `CLAUDE.md` §3 rule 4 says "a change that makes one of them scan fails the
//! bench gate". Until D-0034 there was no bench gate: the workflow step tested
//! for a `benches` directory at the repository **root**, Cargo benches live at
//! `crates/<name>/benches`, and no such directory existed anywhere — so gate 8
//! took its skip arm and reported success without measuring anything. Both
//! defects this change repairs merged through it green.
//!
//! # Why no benchmarking framework
//!
//! `CLAUDE.md` §2 does not forbid a Rust dev-dependency, but a harness would
//! add a tree of them to measure four numbers with `Instant`. `harness = false`
//! makes this an ordinary binary: `cargo bench` runs it, it prints every number
//! it took, and it exits non-zero when a ratio breaches the ceiling. Nothing is
//! reported as passing that was not measured.
//!
//! # What "flat" is measured against
//!
//! `docs/04-invariants.md` sets the ceiling: **1.4×** on dedicated hardware,
//! **3.0×** on shared CI. This file asserts the CI number, because CI is what
//! actually runs, and prints the ratio so a local run can be read against the
//! tighter one.
//!
//! The three rows it prints are the three functions below: `C-01`,
//! `store::bench::header_read_is_flat`; `C-07`,
//! `store::bench::block_seal_is_flat`; and `C-08`,
//! `store::bench::checksum_beats_the_bit_loop`.
//!
//! C-28 and C-29 time the WARM record read, one inside the block verified
//! last; C-BC-01 and C-BC-02 time the COLD one, which verifies its block, and
//! C-BC-02 holds it to its own budget because it does not fit C-29's (D-0914).
//!
//! C-TIX-01 and C-TIX-02 time `BarFile::first_at_or_after` through the `.tix`
//! time index: flat at 1x, 10x and 100x the file, and within the warm record
//! read's budget (D-2329).
//!
//! All arithmetic is integer. `clippy::float_arithmetic` is a workspace lint
//! and a ratio is the one place it would be tempting.

use std::hint::black_box;
use std::time::Instant;

use brutex_core::vendor::Vendor;
use store::block;
use store::crc::crc32c;
use store::file::BarFile;
use store::format::Bar;
use store::format::{BLOCK_LEN, FLAG_CHECKSUMS, HEADER_LEN, SLOT_LEN};
use store::header::Header;
use store::layout::Layout;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

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
/// the cost of the work itself. Measured against a foreign process at 686% CPU
/// during this session, the minimum moved 0.2% across three runs while the
/// median moved 78%.
const TRIALS: u32 = 60;

/// Times one closure, in picoseconds per call, as a minimum over [`TRIALS`].
///
/// Picoseconds because the fastest operation here is a few nanoseconds and an
/// integer ratio of small nanosecond counts is mostly rounding.
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
/// `base` is the 1× cost. A base of zero means the clock could not resolve the
/// operation at all, which is reported as a failure rather than divided by:
/// an unmeasurable baseline makes every ratio meaningless, and reporting a
/// ratio nobody measured is what `CLAUDE.md` §3 rule 6 forbids.
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

/// A zeroed header region holding one genesis commit, `slots` slots long.
fn region(slots: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; slots * SLOT_LEN];
    let Ok(genesis) = Header::genesis(1, 60, FLAG_CHECKSUMS).commit() else {
        return bytes;
    };
    for (dst, src) in bytes.iter_mut().zip(genesis.bytes) {
        *dst = src;
    }
    bytes
}

/// A setup failure this bench cannot measure past, said in the host's words.
///
/// A bench that silently measured a file it failed to fill would report a
/// beautiful ratio over nothing, which is the fallback `CLAUDE.md` §4 bans.
fn refuse(why: &str) -> ! {
    println!("BENCH SETUP FAILED — {why}");
    std::process::exit(1)
}

/// A bars file holding `n` committed records, and the directory that owns it.
///
/// The directory is returned so it outlives the file; dropping it first would
/// unlink the bytes the measurement is about.
fn loaded(name: &str, n: u64) -> (BarFile, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("brutex-bench-{name}-{}", std::process::id()));
    let _ignored = std::fs::remove_dir_all(&root);
    // `open_or_create` never creates the store root (D-1522: a missing root
    // on an unmounted volume must refuse), so the bench makes its own first.
    if let Err(e) = std::fs::create_dir_all(&root) {
        refuse(&format!("the bench root would not create: {e}"));
    }
    let mut file = match BarFile::open_or_create(&root, bench_path(), 7) {
        Ok(f) => f,
        Err(e) => refuse(&format!("the bench file would not open: {e}")),
    };
    // Appended in one batch: `append` requires strictly increasing timestamps,
    // and one call keeps the setup out of the measurement entirely.
    let batch: Vec<Bar> = (0..n)
        .map(|i| {
            let raw = i64::try_from(i).unwrap_or(0);
            Bar {
                ts_micros: JUNE_2024_IST_START.saturating_add(raw.saturating_mul(1_000_000)),
                open: 2_000_000 + raw,
                high: 2_000_100 + raw,
                low: 1_999_900 + raw,
                close: 2_000_050 + raw,
                volume: 1_000,
                open_interest: 0,
            }
        })
        .collect();
    if let Err(e) = file.append(&batch) {
        refuse(&format!("the bench file would not fill: {e}"));
    }
    (file, root)
}

/// 2024-06-01 00:00 IST in epoch microseconds, where every bench bar starts.
///
/// The write boundary refuses a stamp outside the month the path names
/// (D-0915), and the bench used to stamp from the 1970 epoch.
const JUNE_2024_IST_START: i64 = 1_717_180_200_000_000;

/// The path every bench file uses. One month, one symbol, one timeframe.
///
/// THE ONE-SECOND RUNG, because the largest bench file is 100,000 bars and a
/// one-minute month holds at most 44,640: the month admission (D-0915) refuses
/// the rest. 100,000 seconds is under 28 hours. The record geometry is the
/// same at every rung, so what `read_record` costs does not change.
fn bench_path() -> StorePath<'static> {
    match StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: match YearMonth::new(2024, 6) {
            Ok(m) => m,
            Err(_) => refuse("June 2024 is a real month"),
        },
        file: FileKind::Bars,
    }) {
        Ok(p) => p,
        Err(_) => refuse("the bench path is a legal one"),
    }
}

/// The per-read floor: the cheapest possible touch of the same file handle.
///
/// # Why a ratio alone cannot see a regression
///
/// Every row here divides one cost by another of the same operation, and a
/// UNIFORM slowdown cancels in a quotient. An audit measured exactly that
/// elsewhere in this workspace: a mask operation **174x slower passed its
/// crate's ratio rows at 0.98x-1.00x**, because both legs moved together.
///
/// The denominator has to be something that cannot move when `read_record`
/// does. `Layout::offset_of` is the address arithmetic the read is built on —
/// one multiply and one add, no syscall — so the quotient is "how many address
/// computations does one record read cost".
fn floor_ps(layout: Layout) -> u128 {
    cost_ps(2_000, || black_box(layout).offset_of(black_box(7)))
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

/// C-29 — one record read costs a bounded multiple of the address arithmetic.
fn record_read_stays_within_its_budget() -> bool {
    /// Floors allowed per record read.
    ///
    /// A read reaches the filesystem and the floor does not, so this ratio
    /// carries the disk's variance the way `telemetry`'s does — it cannot
    /// resolve a small regression and is not claimed to. What it CAN do is what
    /// no ratio here can: catch a slowdown that moves every file size at once.
    ///
    /// Measured, arm64 laptop, release, three consecutive runs: **199.721,
    /// 191.483, 205.049** floors, at a floor of 1,229–1,270 ps. The spread is
    /// 1.07x — tighter than expected for a path that reaches the filesystem,
    /// because the page is already resident by the second trial.
    ///
    /// **800**, sized on the worst observed with roughly 4x left over — the same
    /// rule the other seven budgets in this workspace apply. A first draft of
    /// this row read 4,000 on the assumption that a syscall would be noisy;
    /// measuring showed it was not, and a budget with 19x headroom is not a
    /// bound, it is a number that would never fire.
    ///
    /// It still refuses the 174x uniform regression: such a read would land near
    /// 35,000 floors and be refused by a factor of 43.
    const ALLOWED: u128 = 800;

    let (file, _d) = loaded("read-budget", 10_000);
    let Ok(layout) = Layout::for_version(2) else {
        refuse("format version 2 has a layout")
    };
    let floor = floor_ps(layout);
    println!("  the per-read floor is {floor} ps — one offset computation");
    let at = cost_ps(2_000, || file.read_record(black_box(9_999)));
    budget(
        "C-29 read_record against the address floor",
        floor,
        at,
        ALLOWED,
    )
}

/// C-28 — reading one record costs the same whatever the file holds.
///
/// # Why this row did not exist until now
///
/// `CLAUDE.md` §3 rule 4 names FIVE operations that must be constant, and **bar
/// lookup is the first of them**. `BarFile::read_record` is documented "Reads
/// one record by index, in O(1)" and, until this row, **nothing in the workspace
/// measured it**. An independent O(1) audit of all thirteen crates found that
/// gap: this crate's bench measured header reads, block sealing and the
/// checksum, and never the read the whole store exists to serve.
///
/// The claim is easy to believe and that is precisely the danger — `offset_of`
/// is multiply-and-add, and the read is a fixed 56 bytes. But "obviously
/// constant" is what `docs/06-limits.md` §7b records four separate defects
/// hiding behind.
///
/// Both ends are read at every size: index 0, and the LAST committed index. A
/// scan that walked to the record would be flat in the first and linear in the
/// second.
fn record_read_is_flat_in_the_file() -> bool {
    let (small, _d1) = loaded("read-small", 1_000);
    let (medium, _d2) = loaded("read-medium", 10_000);
    let (large, _d3) = loaded("read-large", 100_000);

    let first = |f: &BarFile| cost_ps(200, || black_box(f).read_record(black_box(0)));
    let last = |f: &BarFile, n: u64| {
        cost_ps(200, || {
            black_box(f).read_record(black_box(n.saturating_sub(1)))
        })
    };

    let base = first(&small);
    let mut ok = true;
    ok &= ratio("C-28 read_record[0], 10x file", base, first(&medium));
    ok &= ratio("C-28 read_record[0], 100x file", base, first(&large));

    let base_last = last(&small, 1_000);
    ok &= ratio(
        "C-28 read_record[last], 10x file",
        base_last,
        last(&medium, 10_000),
    );
    ok &= ratio(
        "C-28 read_record[last], 100x file",
        base_last,
        last(&large, 100_000),
    );

    // And the two ends of the SAME file cost the same, which is the shape a
    // scan would break first.
    ok &= ratio(
        "C-28 read_record: first against last, same file",
        first(&large),
        last(&large, 100_000),
    );
    ok
}

/// How many distinct checksum blocks one cold measurement cycles through.
///
/// Fourteen because the 1x file holds exactly fourteen blocks (1,000 records
/// at 73 per block), so the 1x row touches every block it has and the larger
/// files are sampled at the same count, spread end to end. Holding the count
/// fixed keeps the footprint of the measurement fixed, so a ratio that moved
/// would be the read moving, not the sample.
const COLD_BLOCKS: usize = 14;

/// Fourteen record indices, each the FIRST record of a different checksum
/// block, spread evenly from block 0 to the tail block of an `n`-record file.
///
/// # Why this shape, and what it closes
///
/// `C-28` and `C-29` re-read one fixed index, and `BarFile` remembers the one
/// block it verified last, so after the first call they time a relaxed load
/// and a 56-byte `pread` — the WARM path. Every random access and every
/// bisection probe pays the COLD path instead: one `pread` of the block, one
/// four-byte `pread` of the sidecar entry and one CRC-32C over up to 4,088
/// bytes, plus an `fstat` on the tail block. Cycling through these indices
/// makes every read land in a block other than the one before it, so every
/// read pays `verify_block_of` in full. The cycle wraps from the tail back to
/// block 0, so the wrap is a block change too.
///
/// The tail block is one in fourteen at every size, so the tail's `fstat` is
/// the same share of every row and cannot masquerade as growth.
fn cold_indices(n: u64) -> [u64; COLD_BLOCKS] {
    let per_block = Layout::V2.records_per_block();
    let blocks = n.div_ceil(per_block);
    let last = u64::try_from(COLD_BLOCKS - 1).unwrap_or(1);
    let mut out = [0u64; COLD_BLOCKS];
    for (slot, step) in out.iter_mut().zip(0u64..) {
        let block = step.saturating_mul(blocks.saturating_sub(1)) / last;
        *slot = block.saturating_mul(per_block);
    }
    // A setup check, not a measurement: two neighbours in one block would let
    // the one-block memory serve the second, and the row would time the warm
    // path while claiming the cold one.
    let mut previous = Layout::V2.block_of(out.last().copied().unwrap_or(0));
    for &index in &out {
        let block = Layout::V2.block_of(index);
        if block == previous || index >= n {
            refuse("a cold index shares its block with its neighbour or is uncommitted");
        }
        previous = block;
    }
    out
}

/// Times a cold record read, in picoseconds per call: every call lands in a
/// different checksum block from the one before it.
fn cold_ps(file: &BarFile, n: u64) -> u128 {
    let at = cold_indices(n);
    let mut next = 0usize;
    cost_ps(280, || {
        let index = at.get(next % COLD_BLOCKS).copied().unwrap_or(0);
        next = next.wrapping_add(1);
        file.read_record(black_box(index))
    })
}

/// C-BC-01 — a COLD record read, one that verifies its checksum block, costs the
/// same whatever the file holds.
///
/// `C-28` measured only the warm path. This row reads indices in fourteen
/// distinct blocks in turn, so every read pays the block `pread`, the sidecar
/// entry and the CRC — the cost a random access or a bisection probe pays —
/// and asserts that cost is flat at 1x, 10x and 100x the record count.
fn cold_record_read_is_flat_in_the_file() -> bool {
    let (small, _d1) = loaded("cold-small", 1_000);
    let (medium, _d2) = loaded("cold-medium", 10_000);
    let (large, _d3) = loaded("cold-large", 100_000);
    let base = cold_ps(&small, 1_000);
    let mut ok = true;
    ok &= ratio(
        "C-BC-01 cold read_record, 10x file",
        base,
        cold_ps(&medium, 10_000),
    );
    ok &= ratio(
        "C-BC-01 cold read_record, 100x file",
        base,
        cold_ps(&large, 100_000),
    );
    ok
}

/// C-BC-02 — a COLD record read costs a bounded multiple of the address floor,
/// under its OWN budget.
fn cold_record_read_stays_within_its_budget() -> bool {
    /// Floors allowed per cold record read — its OWN budget, not C-29's 800.
    ///
    /// **The cold read does not fit 800, and this row is where that is said
    /// rather than hidden.** Measured, `x86_64` shared host (4 cores, 8
    /// concurrent builds), release, 2026-10-02: **2,066.247** floors before
    /// D-0914 removed the per-read heap allocation, and **2,394.687, 2,096.921,
    /// 2,443.033** after it, at floors of 1,407–1,624 ps. The allocation was
    /// not the cost and removing it did not visibly move the number on this
    /// noisy host: the CRC-32C over 4,088 bytes is, and C-07 times that alone
    /// at about 2.45 us, roughly 1,500 floors of the ~2,100–2,450.
    ///
    /// **10,000**, sized on the worst observed (2,443) with roughly 4x left
    /// over, the rule C-29 and the other budgets in this workspace apply. It
    /// still refuses the 174x uniform regression this kind of row exists for:
    /// such a read would land near 360,000 floors.
    const ALLOWED_COLD: u128 = 10_000;

    let (file, _d) = loaded("cold-budget", 10_000);
    let floor = floor_ps(Layout::V2);
    let at = cold_ps(&file, 10_000);
    budget(
        "C-BC-02 cold read_record against the address floor",
        floor,
        at,
        ALLOWED_COLD,
    )
}

/// Fourteen timestamps spread from the first bar of an `n`-bar bench file to
/// its last, each a different index entry from the one before it, so every
/// lookup reads an entry it did not read last.
fn tix_stamps(n: u64) -> [i64; COLD_BLOCKS] {
    let last = u64::try_from(COLD_BLOCKS - 1).unwrap_or(1);
    let mut out = [0i64; COLD_BLOCKS];
    for (slot, step) in out.iter_mut().zip(0u64..) {
        let row = step.saturating_mul(n.saturating_sub(1)) / last;
        let secs = i64::try_from(row).unwrap_or(0);
        *slot = JUNE_2024_IST_START.saturating_add(secs.saturating_mul(1_000_000));
    }
    out
}

/// Times one time-to-row lookup, in picoseconds per call, cycling through
/// [`tix_stamps`]. Each stamp is a bar's own, so each lookup is the index's
/// common case: one 16-byte entry `pread` and no bar read.
fn tix_ps(file: &BarFile, n: u64) -> u128 {
    if file.time_lookup() != store::file::TimeLookup::Indexed {
        refuse("the bench month has no ready time index; the row would time the bisection");
    }
    let at = tix_stamps(n);
    let mut next = 0usize;
    cost_ps(280, || {
        let ts = at.get(next % COLD_BLOCKS).copied().unwrap_or(0);
        next = next.wrapping_add(1);
        file.first_at_or_after(black_box(ts))
    })
}

/// C-TIX-01 — time to row costs the same whatever the file holds. D-2329.
///
/// `BarFile::first_at_or_after` was a bisection (D-1434): its cost grew with
/// `log2(n_valid)`, fourteen probes at a one-minute month. Through the `.tix`
/// index it is one entry read. Timed at 1x, 10x and 100x the record count on
/// the one-second rung; a bisection would be visibly slower at 100x, a scan
/// a hundred times slower.
fn time_lookup_is_flat_in_the_file() -> bool {
    let (small, _d1) = loaded("tix-small", 1_000);
    let (medium, _d2) = loaded("tix-medium", 10_000);
    let (large, _d3) = loaded("tix-large", 100_000);
    let base = tix_ps(&small, 1_000);
    let mut ok = true;
    ok &= ratio(
        "C-TIX-01 first_at_or_after, 10x file",
        base,
        tix_ps(&medium, 10_000),
    );
    ok &= ratio(
        "C-TIX-01 first_at_or_after, 100x file",
        base,
        tix_ps(&large, 100_000),
    );
    ok
}

/// C-TIX-02 — one time-to-row lookup costs a bounded multiple of the address
/// floor, the budget a warm record read is held to. D-2329.
fn time_lookup_stays_within_its_budget() -> bool {
    /// Floors allowed per time lookup: C-29's 800, the warm record read's
    /// budget, because the lookup's common case is one 16-byte `pread` — the
    /// same shape as that read. Measured in the D-2329 commit; see
    /// `docs/04-invariants.md` C-TIX-02.
    const ALLOWED_TIX: u128 = 800;

    let (file, _d) = loaded("tix-budget", 10_000);
    let floor = floor_ps(Layout::V2);
    let at = tix_ps(&file, 10_000);
    budget(
        "C-TIX-02 first_at_or_after against the address floor",
        floor,
        at,
        ALLOWED_TIX,
    )
}

/// The month sizes the p99 rows sweep: 10^3 to 10^6 committed bars.
///
/// 10^6 one-second bars is 11.6 days, inside the June bench month, and well
/// under the one-second ceiling of 2,678,400. Every other row in this file
/// stops at 100,000, so the 10^6 point exists only here.
const P99_SIZES: [u64; 4] = [1_000, 10_000, 100_000, 1_000_000];

/// Rounds per p99 row, and samples per round.
///
/// Each round times every call on its own and keeps every sample; its p99 is
/// the sample at rank 990/1000. The row reports and gates the SMALLEST of the
/// rounds' p99s. A shared runner's scheduler can only ever make a round
/// slower, so one disturbed round cannot breach the row — while a tail that
/// the operation itself pays (one call in fifty doing O(n) work) lands in
/// every round and cannot hide. `max` is the largest sample of all rounds and
/// is printed, never gated: on a shared box it is the scheduler.
const P99_ROUNDS: usize = 5;
const P99_SAMPLES: usize = 20_000;

/// One p99 row's shape at one size, nanoseconds.
struct Tail {
    p50: u128,
    p99: u128,
    max: u128,
}

/// The sample at `permille` thousandths of a sorted round.
fn rank(sorted: &[u128], permille: usize) -> u128 {
    let at = (sorted.len() * permille / 1_000).min(sorted.len().saturating_sub(1));
    sorted.get(at).copied().unwrap_or(0)
}

/// Times `op` once per sample, [`P99_ROUNDS`] rounds of [`P99_SAMPLES`].
///
/// `op` is handed the sample's number so it can address a different record
/// or stamp every call; whatever it returns goes through `black_box`.
fn tail<T>(mut op: impl FnMut(u64) -> T) -> Tail {
    let mut out = Tail {
        p50: u128::MAX,
        p99: u128::MAX,
        max: 0,
    };
    let mut ns = Vec::with_capacity(P99_SAMPLES);
    let mut call = 0u64;
    for _ in 0..P99_ROUNDS {
        ns.clear();
        for _ in 0..P99_SAMPLES {
            let start = Instant::now();
            black_box(op(call));
            ns.push(start.elapsed().as_nanos());
            call = call.wrapping_add(1);
        }
        ns.sort_unstable();
        out.p50 = out.p50.min(rank(&ns, 500));
        out.p99 = out.p99.min(rank(&ns, 990));
        out.max = out.max.max(ns.last().copied().unwrap_or(0));
    }
    out
}

/// A fixed-seed generator for the sampled positions, so two runs time the
/// same sequence of records and stamps. `SplitMix64`'s finaliser.
fn mix(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The record index sample `call` reads in an `n`-record file: a uniformly
/// drawn record in a block OTHER than the one sample `call - 1` read, so the
/// handle's one-block memory never serves it and every sample pays the cold
/// verify — the path a random access and a bisection probe pay.
fn cold_index(n: u64, call: u64) -> u64 {
    let per_block = Layout::V2.records_per_block();
    let blocks = n.div_ceil(per_block).max(2);
    let previous = if call == 0 {
        blocks - 1
    } else {
        mix(call - 1) % blocks
    };
    let mut block = mix(call) % blocks;
    if block == previous {
        block = (block + 1) % blocks;
    }
    let index = block * per_block + mix(call ^ 0x5555) % per_block;
    index.min(n - 1)
}

/// The stamp sample `call` looks up in an `n`-bar bench file: uniform in
/// microseconds over the bars' whole span, so most samples fall between two
/// bars and none repeats the entry the one before it read.
fn tail_stamp(n: u64, call: u64) -> i64 {
    let span = n.saturating_mul(1_000_000);
    let offset = i64::try_from(mix(call) % span).unwrap_or(0);
    JUNE_2024_IST_START.saturating_add(offset)
}

/// Prints one size's p50 / p99 / max and gates its p99 against the 10^3 one.
fn tail_row(label: &str, n: u64, base_p99: u128, at: &Tail) -> bool {
    println!(
        "  {label:<44} n={n:>9}  p50 {:>7} ns  p99 {:>7} ns  max {:>9} ns",
        at.p50, at.p99, at.max
    );
    if n == P99_SIZES[0] {
        return true;
    }
    // `ratio` prints picoseconds; the samples are nanoseconds.
    ratio(
        &format!("{label} p99, {n} against 1000"),
        base_p99.saturating_mul(1_000),
        at.p99.saturating_mul(1_000),
    )
}

/// O1P-01 and O1P-02 — bar lookup and time lookup are flat AT p99, from 10^3
/// to 10^6 bars.
///
/// # Why these rows exist
///
/// Every other row in this file is a minimum over trials of a MEAN. A mean
/// cannot see a tail, and the minimum over trials discards whichever trial
/// paid one: a read that cost O(n) on one call in fifty would move C-BC-01 by
/// a fraction and stay green. `CLAUDE.md` §3 rule 4 names bar lookup first,
/// and `docs/06-limits.md` §1 says each named operation "is O(1) and each is
/// measured by gate 8" — measured by a statistic that could not fail on a
/// tail. These rows apply the SAME [`CEILING_PERMILLE`] to the SAME flatness
/// claim, measured at p99 (D-3300). The C-T-01b row in `crates/telemetry`
/// did this for `emit`; nothing did it for the store.
///
/// O1P-01 is the cold `read_record` (a different block every call); O1P-02 is
/// `first_at_or_after` through the `.tix` index, at random microseconds.
fn lookups_are_flat_at_p99() -> bool {
    let mut ok = true;
    let mut base_read = 0u128;
    let mut base_time = 0u128;
    for (step, n) in P99_SIZES.into_iter().enumerate() {
        let (file, _dir) = loaded(&format!("p99-{step}"), n);
        if file.time_lookup() != store::file::TimeLookup::Indexed {
            refuse("a p99 bench month has no ready time index");
        }
        let read = tail(|call| file.read_record(black_box(cold_index(n, call))));
        let time = tail(|call| file.first_at_or_after(black_box(tail_stamp(n, call))));
        if step == 0 {
            base_read = read.p99;
            base_time = time.p99;
        }
        ok &= tail_row("O1P-01 cold read_record", n, base_read, &read);
        ok &= tail_row("O1P-02 first_at_or_after", n, base_time, &time);
    }
    ok
}

/// C-01 — reading the header costs the same whatever region it is handed.
///
/// The region a caller passes may be a whole read-only mapping of the file, so
/// "the region grew" is "the file grew". [`Header::read_region`] is bounded by
/// `MAX_SLOTS` positions rather than by the region's length; before that bound
/// existed it auditioned every aligned run of record bytes as a header.
fn header_read_is_flat() -> bool {
    let file_len = HEADER_LEN;
    // 1x is the real header region; 10x and 100x are what a mapping looks like.
    let one = region(512);
    let ten = region(5_120);
    let hundred = region(51_200);
    let time = |r: &[u8]| cost_ps(200, || Header::read_region(black_box(r), file_len));
    let base = time(&one);
    let a = ratio("C-01 header read, 10x region", base, time(&ten));
    let b = ratio("C-01 header read, 100x region", base, time(&hundred));
    a && b
}

/// C-07 — sealing one block costs the same whatever the file holds.
///
/// This is the per-operation claim `docs/02-store-format.md` makes about
/// verification: one block's checksum, not a file scan. `n_valid` is the whole
/// file's record count and the block index is fixed, so a cost that tracked
/// `n_valid` would mean the verifier had started reading past its block.
fn block_seal_is_flat() -> bool {
    let bytes = vec![0x5Au8; usize::try_from(BLOCK_LEN).unwrap_or(0)];
    let records_per_block = BLOCK_LEN / 56;
    let time = |n_valid: u64| {
        cost_ps(200, || {
            block::seal(Layout::V2, n_valid, 0, black_box(&bytes))
        })
    };
    let base = time(records_per_block);
    let a = ratio(
        "C-07 block seal, 10x file",
        base,
        time(records_per_block * 10),
    );
    let b = ratio(
        "C-07 block seal, 100x file",
        base,
        time(records_per_block * 100),
    );
    a && b
}

/// The bit-by-bit kernel this store ran until D-0032.
///
/// Kept here so the comparison below is measured in the same process, on the
/// same machine, under the same load — the only shape of absolute-speed
/// assertion that means anything on a shared CI runner.
fn crc32c_bitwise(bytes: &[u8]) -> u32 {
    const POLYNOMIAL: u32 = 0x82F6_3B78;
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8u8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (POLYNOMIAL & mask);
        }
    }
    !crc
}

/// C-08 — the block checksum stays far away from the bit-by-bit kernel.
///
/// A ratio, not a nanosecond ceiling. An absolute threshold would have to be
/// guessed for hardware this repository has never measured on — CI is `x86_64`
/// and the operator's machine is `aarch64` — and a guessed threshold is either
/// red for no reason or green for every regression. Both kernels here scale
/// with the machine, so their ratio does not.
///
/// Measured on an Apple M4 Pro over one 4,088-byte block: 9.4×. The floor is
/// set at 3× so ordinary hardware variation cannot trip it, while a return to
/// walking bit by bit — which is what happened, and what this exists to catch —
/// cannot pass it.
fn checksum_beats_the_bit_loop() -> bool {
    const FLOOR_PERMILLE: u128 = 3_000;
    let bytes = vec![0x5Au8; usize::try_from(BLOCK_LEN).unwrap_or(0)];
    let fast = cost_ps(200, || crc32c(black_box(&bytes)));
    let slow = cost_ps(20, || crc32c_bitwise(black_box(&bytes)));
    if fast == 0 {
        println!("  C-08 checksum vs bit loop                    UNMEASURABLE");
        return false;
    }
    let permille = slow * 1_000 / fast;
    let ok = permille >= FLOOR_PERMILLE;
    println!(
        "  {:<44} {:>9} ps vs {:>9} ps  speedup {}.{:03}x  {}",
        "C-08 checksum vs the bit loop it replaced",
        fast,
        slow,
        permille / 1_000,
        permille % 1_000,
        if ok { "ok" } else { "BREACH" }
    );
    ok
}

fn main() {
    println!("gate 8 — crates/store, ceiling {CEILING_PERMILLE} permille");
    let mut ok = true;
    ok &= header_read_is_flat();
    ok &= block_seal_is_flat();
    ok &= checksum_beats_the_bit_loop();
    ok &= record_read_is_flat_in_the_file();
    ok &= record_read_stays_within_its_budget();
    ok &= cold_record_read_is_flat_in_the_file();
    ok &= cold_record_read_stays_within_its_budget();
    ok &= time_lookup_is_flat_in_the_file();
    ok &= time_lookup_stays_within_its_budget();
    ok &= lookups_are_flat_at_p99();
    if ok {
        println!("all ratios within the ceiling");
    } else {
        println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
        std::process::exit(1);
    }
}
