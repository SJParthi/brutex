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
//! C-VD-01, C-VD-02 and C-VD-04 time the verdict store (D-4808, D-4844): a
//! lookup cold and warm at 1x, 10x and 50x a day's records, on colliding
//! keys and over a year of days, with the 100x day's `table_too_large`
//! refusal pinned (L-IA-33); the year index read; and the commit alone, on
//! Zerodha-sized and options-sized days, each with p50, p99 and max printed
//! beside its ratio and never gated.
//!
//! C-RP-01 times `repair::repair_month`, which is NOT O(1) and says so: one
//! repair reads the served month whole and writes a whole successor. It is
//! timed at a month the size of a real one-second month and at a month filled
//! to its grid ceiling, the cost of one merged bar is held flat between the
//! two, the repaired full month's time lookups are held to C-TIX-02's budget,
//! and the process's peak resident set is read where the host keeps one
//! (D-4875 (1), D-4898; `docs/06-limits.md` L-IA-26).
//!
//! C-SF-01 to C-SF-03 time R3-16's two shared file doors, the reading door
//! and the staging create, in `ratio/safe_files.rs`.
//!
//! All arithmetic is integer. `clippy::float_arithmetic` is a workspace lint
//! and a ratio is the one place it would be tempting.

use std::hint::black_box;
use std::time::Instant;

#[path = "ratio/safe_files.rs"]
mod safe_files;

#[path = "ratio/months.rs"]
mod months;

use brutex_core::vendor::Vendor;
use store::block;
use store::crc::crc32c;
use store::file::BarFile;
use store::format::Bar;
use store::format::{BLOCK_LEN, FLAG_CHECKSUMS, HEADER_LEN, SLOT_LEN};
use store::header::Header;
use store::layout::Layout;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::provenance::Day;
use store::repair::{self, MonthRepair};
use store::verdict;

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
    let lookup = file.time_lookup();
    if lookup != store::file::TimeLookup::Indexed {
        refuse(&format!(
            "the bench month of {} bars has no ready time index ({lookup:?}); the row would \
             time the bisection",
            file.records()
        ));
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
///
/// **1x is the smallest month that has a `.tix`,** not a round number: a
/// month under [`store::time_index::INDEX_MIN_ROWS`] bars is given none by the
/// small-month rule (D-4933) and its lookups bisect, which is not what this
/// row times. The 1,000-bar month this row used to start from is such a month
/// since that rule, and [`tix_ps`] refused it, so the row and every row after
/// it never ran. The sizes follow the threshold, so they cannot fall under it
/// again: 1,024, 10,240 and 102,400 one-second bars today, all inside June.
fn time_lookup_is_flat_in_the_file() -> bool {
    let one = store::time_index::INDEX_MIN_ROWS;
    let ten = one.saturating_mul(10);
    let hundred = one.saturating_mul(100);
    let (small, _d1) = loaded("tix-small", one);
    let (medium, _d2) = loaded("tix-medium", ten);
    let (large, _d3) = loaded("tix-large", hundred);
    let base = tix_ps(&small, one);
    let mut ok = true;
    ok &= ratio(
        "C-TIX-01 first_at_or_after, 10x file",
        base,
        tix_ps(&medium, ten),
    );
    ok &= ratio(
        "C-TIX-01 first_at_or_after, 100x file",
        base,
        tix_ps(&large, hundred),
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

// ══ The verdict store (D-4808, D-4844; IA-5): C-VD-01, C-VD-02, C-VD-04 ═════
//
// Every key here is a synthetic `ZZ` name no exchange lists, and every
// verdict is invented. The files live under the temporary directory, so the
// numbers are that disk's: run with TMPDIR on the store's volume to measure
// the store's disk.

/// Single-operation latencies, nanoseconds: p50, p99 and max over `n` calls.
/// Printed beside each ratio, never gated: a cold lookup opens two files and
/// one slow open is the scheduler's, which the ratio's minimum discards and
/// these do not.
fn spread(label: &str, n: u32, mut op: impl FnMut()) {
    spread_prepared(label, n, || (), |()| op());
}

/// As [`spread`], but each call's input is made by `prepare` outside the
/// timer, so only `op` is timed (C-VD-04 times the commit alone, never the
/// building of the generation it is handed).
fn spread_prepared<T>(label: &str, n: u32, mut prepare: impl FnMut() -> T, mut op: impl FnMut(T)) {
    let mut ns: Vec<u128> = (0..n)
        .map(|_| {
            let input = prepare();
            let start = Instant::now();
            op(input);
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
        "  {label:<56} p50 {:>9} ns  p99 {:>9} ns  max {:>10} ns  n={n}",
        at(50),
        at(99),
        ns.last().copied().unwrap_or(0)
    );
}

/// A fresh store root for one verdict bench, removed first.
fn vd_root(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("brutex-bench-vd-{name}-{}", std::process::id()));
    let _ignored = std::fs::remove_dir_all(&root);
    if let Err(e) = std::fs::create_dir_all(&root) {
        refuse(&format!("the verdict bench root would not create: {e}"));
    }
    root
}

/// The bench's series: GDFL options, one-second rung.
fn vd_series(root: &std::path::Path) -> verdict::Series<'_> {
    verdict::Series {
        root,
        feed: verdict::Feed::Gdfl,
        exchange: brutex_core::instrument::Exchange::Nse,
        segment: brutex_core::instrument::Segment::Fno,
        timeframe: Timeframe::SECOND_1,
    }
}

/// A Zerodha series: cash shares, one-minute rung.
fn vd_zerodha(root: &std::path::Path) -> verdict::Series<'_> {
    verdict::Series {
        root,
        feed: verdict::Feed::Zerodha,
        exchange: brutex_core::instrument::Exchange::Nse,
        segment: brutex_core::instrument::Segment::Cash,
        timeframe: Timeframe::MINUTE_1,
    }
}

/// A synthetic instrument key about as long as a real option key.
fn vd_name(i: usize) -> String {
    format!("NSE-FNO-ZZSYN{:02}-{:07}-CE-{:09}", i % 37, i, i * 7 + 3)
}

fn vd_key(i: usize) -> verdict::Key {
    verdict::Key::instrument(&vd_name(i)).unwrap_or_else(|_| refuse("a synthetic key fits"))
}

/// The `i`-th element of a non-empty slice, cycling.
fn nth<T>(xs: &[T], i: usize) -> &T {
    xs.get(i % xs.len().max(1))
        .unwrap_or_else(|| refuse("a bench cycles over an empty list"))
}

/// A day: 2031-01-01 plus `offset` days, within 2031.
fn vd_day(offset: u16) -> Day {
    let (mut month, mut day) = (1u8, offset + 1);
    let lengths = [31u16, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for len in lengths {
        if day <= len {
            break;
        }
        day -= len;
        month += 1;
    }
    Day::new(2031, month, u8::try_from(day).unwrap_or(1)).unwrap_or_else(|| refuse("a real day"))
}

/// One generation of `n` synthetic records, most `match`, every tenth
/// another code.
fn vd_generation(n: usize, day: Day, run: u64) -> verdict::Generation {
    let entries = (0..n)
        .map(|i| {
            let code = if i % 10 == 0 {
                *nth(&verdict::Code::ALL, i / 10)
            } else {
                verdict::Code::Match
            };
            let mut facts = verdict::Facts::of(code);
            facts.source_units = u32::try_from(i).unwrap_or(0);
            verdict::Entry {
                key: vd_key(i),
                facts,
                details: Vec::new(),
            }
        })
        .collect();
    verdict::Generation {
        day,
        run,
        written_at: 1_900_000_000,
        verifier: 1,
        mode: verdict::Mode::Cold,
        rule_set: 0,
        origin: [0; 16],
        day_block: verdict::DayBlock::of(verdict::Code::Match),
        day_details: Vec::new(),
        entries,
        carry: verdict::Carry::WholeDay,
    }
}

/// Commits one day of `n` records.
fn vd_commit(root: &std::path::Path, n: usize, day: Day, run: u64) -> verdict::Committed {
    match verdict::commit(&vd_series(root), &vd_generation(n, day, run)) {
        Ok(done) => done,
        Err(e) => refuse(&format!("a verdict day of {n} would not commit: {e}")),
    }
}

/// An image's table capacity and its bytes on disk.
fn vd_size(done: &verdict::Committed) -> (u32, u64) {
    let bytes = std::fs::metadata(&done.path).map_or(0, |m| m.len());
    let capacity = verdict::read_image(&done.path).map_or(0, |img| img.header.capacity);
    (capacity, bytes)
}

/// The keys a lookup row cycles through: present keys spread over the day,
/// so successive lookups land on different table windows and records.
fn vd_keys(n: usize) -> Vec<verdict::Key> {
    (0..64usize).map(|i| vd_key(i * (n - 1) / 63)).collect()
}

/// Keys of a day of `n` records that share a home slot with at least two
/// others at `capacity`: the colliding-key fixture.
fn vd_colliding(n: usize, capacity: u32) -> Vec<verdict::Key> {
    let mask = u64::from(capacity.saturating_sub(1));
    let mut by_home: std::collections::HashMap<u64, Vec<usize>> =
        std::collections::HashMap::with_capacity(n);
    for i in 0..n {
        by_home
            .entry(verdict::key_hash(vd_name(i).as_bytes()) & mask)
            .or_default()
            .push(i);
    }
    let mut out: Vec<usize> = by_home
        .into_values()
        .filter(|group| group.len() >= 3)
        .flatten()
        .collect();
    out.sort_unstable();
    out.truncate(64);
    if out.is_empty() {
        refuse("the colliding-key fixture found no crowded home slot");
    }
    out.into_iter().map(vd_key).collect()
}

/// One lookup of `key`, refusing a miss: a row that timed a miss would be
/// timing the no-record arm while claiming the record arm.
fn vd_lookup(
    reader: &mut verdict::Reader,
    root: &std::path::Path,
    day: Day,
    key: &verdict::Key,
) -> u32 {
    match reader.lookup(&vd_series(root), day, key) {
        Ok(found)
            if matches!(
                found.answer.as_ref().map(|a| &a.found),
                Some(verdict::Found::Record { .. })
            ) && found.cost.reads <= 5
                && found.cost.opens <= 2 =>
        {
            found.cost.reads
        }
        Ok(found) if found.cost.reads > 5 || found.cost.opens > 2 => {
            refuse("a lookup exceeded 5 reads or 2 opens")
        }
        Ok(_) => refuse("a present key was not found"),
        Err(e) => refuse(&format!("a lookup was refused: {e}")),
    }
}

/// C-VD-01, the ceiling — a day of 275,800 records (100x 2,758) is past
/// format version 1's record ceiling (L-IA-33): it is refused
/// `table_too_large` by name with nothing written, not even a directory.
fn verdict_ceiling_is_refused_whole() -> bool {
    let refused_root = vd_root("refused");
    let refused = verdict::commit(
        &vd_series(&refused_root),
        &vd_generation(275_800, vd_day(62), 40),
    );
    let nothing = std::fs::read_dir(&refused_root).map_or(0, Iterator::count) == 0;
    let ok = match refused {
        Err(verdict::VerdictError::TableTooLarge { records }) if records == 275_800 && nothing => {
            println!(
                "  C-VD-01 day of  275800 records (100x): refused table_too_large, nothing written"
            );
            true
        }
        other => {
            println!(
                "  C-VD-01 100x day: expected table_too_large with nothing written, got {other:?}"
            );
            false
        }
    };
    let _ignored = std::fs::remove_dir_all(&refused_root);
    ok
}

/// C-VD-01 — one verdict lookup costs the same whatever the day holds: cold
/// (a new reader, so both files are opened) and warm (cached handles), at
/// 1x (2,758 records), 10x and 50x, and on keys that share a home slot.
/// 100x is past the format (`verdict_ceiling_is_refused_whole`).
fn verdict_lookup_is_flat_in_the_day() -> bool {
    let mut ok = true;
    let sizes = [2_758usize, 27_580, 137_900];
    let day = vd_day(62);
    let mut roots = Vec::with_capacity(sizes.len());
    for (i, n) in sizes.iter().enumerate() {
        let root = vd_root(&format!("lookup-{i}"));
        let done = vd_commit(&root, *n, day, 41);
        let (capacity, bytes) = vd_size(&done);
        println!(
            "  C-VD-01 day of {n:>7} records: table capacity {capacity:>8}, image {bytes:>10} bytes"
        );
        roots.push((root, *n, capacity));
    }
    let cold = |root: &std::path::Path, keys: &[verdict::Key]| {
        let mut next = 0usize;
        cost_ps(40, || {
            let key = nth(keys, next);
            next = next.wrapping_add(1);
            vd_lookup(&mut verdict::Reader::new(), root, day, key)
        })
    };
    let warm = |root: &std::path::Path, keys: &[verdict::Key]| {
        let mut reader = verdict::Reader::new();
        let mut next = 0usize;
        cost_ps(400, || {
            let key = nth(keys, next);
            next = next.wrapping_add(1);
            vd_lookup(&mut reader, root, day, key)
        })
    };
    let (base_root, base_n, base_capacity) = nth(&roots, 0);
    let base_keys = vd_keys(*base_n);
    let base_cold = cold(base_root, &base_keys);
    let base_warm = warm(base_root, &base_keys);
    for (root, n, _) in roots.iter().skip(1) {
        let x = n / base_n;
        let keys = vd_keys(*n);
        ok &= ratio(
            &format!("C-VD-01 cold lookup, {x}x day"),
            base_cold,
            cold(root, &keys),
        );
        ok &= ratio(
            &format!("C-VD-01 warm lookup, {x}x day"),
            base_warm,
            warm(root, &keys),
        );
    }
    let colliding = vd_colliding(*base_n, *base_capacity);
    ok &= ratio(
        "C-VD-01 warm lookup, colliding keys at 1x",
        base_warm,
        warm(base_root, &colliding),
    );
    for (root, n, capacity) in &roots {
        for (what, keys) in [
            ("spread", vd_keys(*n)),
            ("colliding", vd_colliding(*n, *capacity)),
        ] {
            let mut next = 0usize;
            spread(
                &format!("C-VD-01 cold lookup, {n} records, {what} keys"),
                2_000,
                || {
                    let key = nth(&keys, next);
                    next = next.wrapping_add(1);
                    black_box(vd_lookup(&mut verdict::Reader::new(), root, day, key));
                },
            );
            let mut reader = verdict::Reader::new();
            spread(
                &format!("C-VD-01 warm lookup, {n} records, {what} keys"),
                20_000,
                || {
                    let key = nth(&keys, next);
                    next = next.wrapping_add(1);
                    black_box(vd_lookup(&mut reader, root, day, key));
                },
            );
        }
    }

    for (root, _, _) in roots {
        let _ignored = std::fs::remove_dir_all(root);
    }
    ok
}

/// C-VD-01 over a year: 248 days of 2,758 records, looked up in a scattered
/// order through one reader whose 64 handles cannot hold the year, so most
/// lookups open both files. p50, p99 and max only; a lookup past 5 reads
/// or 2 opens, or one that misses its record, stops the bench (`refuse`).
fn verdict_lookup_over_a_year() {
    let year = vd_root("year");
    for d in 0..248u16 {
        vd_commit(&year, 2_758, vd_day(d), 1_000 + u64::from(d));
    }
    let keys = vd_keys(2_758);
    let mut reader = verdict::Reader::new();
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    spread(
        "C-VD-01 lookups over a year (248 days x 2,758)",
        20_000,
        || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let d = u16::try_from((state >> 33) % 248).unwrap_or(0);
            let key = nth(&keys, usize::try_from((state >> 13) % 64).unwrap_or(0));
            black_box(vd_lookup(&mut reader, &year, vd_day(d), key));
        },
    );
    let _ignored = std::fs::remove_dir_all(&year);
}

/// C-VD-02 — reading a year index costs the same whatever the year holds:
/// one committed day against 248. The `.vdi` is fixed-size, so a cost that
/// grew would mean the read had started walking the images.
fn year_index_read_is_flat() -> bool {
    let one = vd_root("vdi-one");
    vd_commit(&one, 4, vd_day(0), 7);
    let full = vd_root("vdi-full");
    for d in 0..248u16 {
        vd_commit(&full, 4, vd_day(d), 7 + u64::from(d));
    }
    let time = |root: &std::path::Path| {
        let mut reader = verdict::Reader::new();
        cost_ps(400, || match reader.year(&vd_series(root), 2031) {
            Ok(Some(index)) => index.commit,
            _ => refuse("the year index would not read"),
        })
    };
    let ok = ratio(
        "C-VD-02 year index read, 248 days vs 1",
        time(&one),
        time(&full),
    );
    let mut reader = verdict::Reader::new();
    spread("C-VD-02 year index read (warm handle)", 20_000, || {
        black_box(reader.year(&vd_series(&full), 2031).is_ok());
    });
    let _ignored = std::fs::remove_dir_all(one);
    let _ignored = std::fs::remove_dir_all(full);
    ok
}

/// C-VD-04 — the commit alone (each generation is built before the timer
/// starts): O(n log n) for the sort plus three `sync_all`s (L-IA-4), so its
/// cost grows with the day and only its cost per record is gated: at 10x it
/// stays within the ceiling of the cost per record at 1x, which an O(n²)
/// build would break. Then p50, p99 and max of the commit on synthetic
/// images the size of one Zerodha month's days (21 days of 343 instruments,
/// 1x, and of 3,430, 10x) and of the average (10,428) and largest (26,820)
/// options days, each with its table capacity and image bytes (L-IA-7).
fn generation_write_stays_n_log_n() -> bool {
    let root = vd_root("write");
    let mut run = 1u64;
    let mut per_record = |n: usize, reps: u32| {
        let mut best = u128::MAX;
        for _ in 0..reps {
            run += 1;
            let generation = vd_generation(n, vd_day(10), run);
            let start = Instant::now();
            if let Err(e) = verdict::commit(&vd_series(&root), &generation) {
                refuse(&format!("a generation would not commit: {e}"));
            }
            let ps = start.elapsed().as_nanos() * 1_000 / u128::try_from(n).unwrap_or(1);
            best = best.min(ps);
        }
        best
    };
    let base = per_record(2_758, 12);
    let ok = ratio(
        "C-VD-04 commit per record, 10x day",
        base,
        per_record(27_580, 4),
    );

    // One Zerodha month's days: 21 trading days, one image each, at 343
    // instruments and at 10x. Each day is committed once, as a merge would.
    let zerodha = vd_root("write-zerodha");
    for (n, label) in [(343usize, "1x"), (3_430, "10x")] {
        let mut d = 0u16;
        let mut sizes = (0u32, 0u64);
        spread_prepared(
            &format!("C-VD-04 commit, Zerodha day of {n} instruments ({label})"),
            21,
            || {
                d += 1;
                vd_generation(
                    n,
                    vd_day(d + if n > 343 { 100 } else { 0 }),
                    500 + u64::from(d),
                )
            },
            |generation| match verdict::commit(&vd_zerodha(&zerodha), &generation) {
                Ok(done) => sizes = vd_size(&done),
                Err(e) => refuse(&format!("a Zerodha-sized day would not commit: {e}")),
            },
        );
        println!(
            "  C-VD-04 Zerodha day of {n:>6} records: table capacity {:>7}, image {:>9} bytes",
            sizes.0, sizes.1
        );
    }
    let _ignored = std::fs::remove_dir_all(zerodha);

    let mut next = 100_000u64;
    for (n, offset, reps) in [(10_428usize, 20u16, 10u32), (26_820, 21, 10)] {
        let mut sizes = (0u32, 0u64);
        spread_prepared(
            &format!("C-VD-04 commit, options day of {n} records"),
            reps,
            || {
                next += 1;
                vd_generation(n, vd_day(offset), next)
            },
            |generation| match verdict::commit(&vd_series(&root), &generation) {
                Ok(done) => sizes = vd_size(&done),
                Err(e) => refuse(&format!("an options-sized day would not commit: {e}")),
            },
        );
        println!(
            "  C-VD-04 options day of {n:>6} records: table capacity {:>7}, image {:>9} bytes",
            sizes.0, sizes.1
        );
    }
    let _ignored = std::fs::remove_dir_all(root);
    ok
}

/// 2024-07-01 00:00 IST in epoch microseconds: June's start and June's 30
/// days. July, because C-RP-01 fills a month to the most any month holds and
/// that takes 31 days.
const JULY_2024_IST_START: i64 = JUNE_2024_IST_START + 30 * 86_400 * 1_000_000;

/// The month C-RP-01 repairs: an invented index's one-second July.
fn repair_path() -> StorePath<'static> {
    match StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "FIXTUREIDX",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: match YearMonth::new(2024, 7) {
            Ok(m) => m,
            Err(_) => refuse("July 2024 is a real month"),
        },
        file: FileKind::Bars,
    }) {
        Ok(p) => p,
        Err(_) => refuse("the repair bench path is a legal one"),
    }
}

/// `count` invented one-second bars from `first` seconds into July.
fn july_seconds(first: i64, count: i64) -> Vec<Bar> {
    (first..first.saturating_add(count))
        .map(|second| {
            let price = 123_400 + second % 97;
            Bar {
                ts_micros: JULY_2024_IST_START.saturating_add(second.saturating_mul(1_000_000)),
                open: price,
                high: price + 4,
                low: price - 4,
                close: price + 1,
                volume: 0,
                open_interest: store::format::OI_NULL,
            }
        })
        .collect()
}

/// One month C-RP-01 repairs: the runs of seconds it holds, each `(first
/// second of July, count)`, and the one run it lacks.
struct RepairShape {
    name: &'static str,
    held: Vec<(i64, i64)>,
    offered: (i64, i64),
}

impl RepairShape {
    /// A month the size of a real one-second index month: 19 sessions, 18 of
    /// the regular 22,500 seconds and one cut to 20,817, which is 425,817
    /// held bars (the design's figure for one NIFTY month), and the session
    /// of the tenth day, which it lacks.
    fn session_month() -> Self {
        const OPEN: i64 = 555 * 60;
        let held = (0..20)
            .filter(|day| *day != 9)
            .map(|day| (day * 86_400 + OPEN, if day == 19 { 20_817 } else { 22_500 }))
            .collect();
        Self {
            name: "session-month",
            held,
            offered: (9 * 86_400 + OPEN, 22_500),
        }
    }

    /// A month filled to its grid ceiling: every second of 30 of July's 31
    /// days held, and every second of the day it lacks offered. 2,678,400
    /// merged rows, the most `repair::row_ceiling` allows any path.
    fn full_month() -> Self {
        Self {
            name: "full-month",
            held: (0..31)
                .filter(|day| *day != 17)
                .map(|day| (day * 86_400, 86_400))
                .collect(),
            offered: (17 * 86_400, 86_400),
        }
    }

    fn merged(&self) -> u128 {
        let held: i64 = self.held.iter().map(|run| run.1).sum();
        u128::try_from(held.saturating_add(self.offered.1)).unwrap_or(0)
    }

    /// Writes the held month once, into a store of its own that every trial
    /// then copies. A run at a time, so that nothing the setup holds is as
    /// large as what the repair holds: the peak resident set this process
    /// reaches is the repair's.
    fn template(&self) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "brutex-bench-repair-{}-template-{}",
            self.name,
            std::process::id()
        ));
        let _ignored = std::fs::remove_dir_all(&root);
        if let Err(e) = std::fs::create_dir_all(&root) {
            refuse(&format!("the repair bench root would not create: {e}"));
        }
        let mut writer = match BarFile::open_or_create(&root, repair_path(), 7) {
            Ok(f) => f,
            Err(e) => refuse(&format!("the repair bench month would not open: {e}")),
        };
        for (first, count) in &self.held {
            if let Err(e) = writer.append(&july_seconds(*first, *count)) {
                refuse(&format!("the repair bench month would not fill: {e}"));
            }
        }
        root
    }

    /// Copies the held month into a fresh store, repairs it once, and returns
    /// the repair's own time in nanoseconds with the store's root. Only
    /// `repair_month` is inside the clock. A fresh store each time, because a
    /// repair is a write and cannot be repeated on the month it repaired.
    fn repair_once(&self, template: &std::path::Path) -> (u128, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "brutex-bench-repair-{}-{}",
            self.name,
            std::process::id()
        ));
        let _ignored = std::fs::remove_dir_all(&root);
        if let Err(e) = copy_tree(template, &root) {
            refuse(&format!("the held month would not copy: {e}"));
        }
        let path = repair_path();
        let offered = july_seconds(self.offered.0, self.offered.1);
        let policy = repair::Policy {
            open_interest_applies: false,
            basis_witness: false,
        };
        let start = Instant::now();
        let outcome = black_box(repair::repair_month(&root, path, 7, &offered, policy));
        let nanos = start.elapsed().as_nanos();
        match outcome {
            Ok(MonthRepair::Repaired { held, added, .. })
                if u128::from(held.saturating_add(added)) == self.merged()
                    && u128::from(added) == u128::try_from(self.offered.1).unwrap_or(0) => {}
            other => refuse(&format!(
                "the {} repair did not add its day: {other:?}",
                self.name
            )),
        }
        (nanos, root)
    }

    /// `trials` repairs, each of a fresh store. Prints n, p50, p99 and max,
    /// and returns the fastest, in picoseconds a merged bar, with the last
    /// trial's store left on disk for the caller to read and remove.
    ///
    /// The p99 of n samples is the sample at rank `ceil(0.99 n)`, which is
    /// the slowest for any n under 100: the line prints n so that is read.
    fn measure(&self, trials: usize) -> (u128, std::path::PathBuf) {
        let template = self.template();
        let mut nanos = Vec::with_capacity(trials);
        let mut last = std::path::PathBuf::new();
        for trial in 0..trials {
            let (took, root) = self.repair_once(&template);
            nanos.push(took);
            if trial + 1 == trials {
                last = root;
            } else {
                let _ignored = std::fs::remove_dir_all(&root);
            }
        }
        let _ignored = std::fs::remove_dir_all(&template);
        nanos.sort_unstable();
        let at = |rank: usize| nanos.get(rank.saturating_sub(1)).copied().unwrap_or(0);
        let (fastest, p50, p99, max) = (
            at(1),
            at(trials.div_ceil(2)),
            at((trials * 99).div_ceil(100)),
            at(trials),
        );
        let merged = self.merged().max(1);
        println!(
            "  C-RP-01 repair_month, {:<13} {:>9} merged rows  n={trials} p50 {} us  p99 {} us  max {} us  ({} ns a row at the fastest)",
            self.name,
            merged,
            p50 / 1_000,
            p99 / 1_000,
            max / 1_000,
            fastest / merged,
        );
        (fastest.saturating_mul(1_000) / merged, last)
    }
}

/// Copies every file beneath `from` to the same place beneath `to`.
fn copy_tree(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// The most this process's resident set has been, in bytes, where the host
/// keeps that figure: Linux's `VmHWM` line of `/proc/self/status`. `None`
/// anywhere else. Read with `std` alone: this workspace takes no `libc`, and
/// `getrusage` is a foreign call (`unsafe`, which gate 5 counts).
fn peak_resident_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find_map(|l| l.strip_prefix("VmHWM:"))?;
    let kib: u64 = line.trim().strip_suffix("kB")?.trim().parse().ok()?;
    Some(kib.saturating_mul(1_024))
}

/// Asks the host to forget the peak it has recorded and start again from what
/// this process holds now, so that the figure [`peak_resident_bytes`] reads
/// afterwards is the peak of what ran in between and not of an earlier row of
/// this bench. Linux: `5` written to `/proc/self/clear_refs` resets `VmHWM`
/// to the current resident set. `false` where there is no such file or the
/// host refuses the write; the peak read then is the whole process's, and the
/// line that prints it says so.
fn restart_peak_resident() -> bool {
    std::fs::write("/proc/self/clear_refs", "5").is_ok()
}

/// What five repairs of a one-second month filled to its grid ceiling may
/// hold at their peak: 640 MiB. `docs/06-limits.md` L-IA-26.
///
/// Measured, not summed from type sizes. One such repair in a process of its
/// own peaked at 537,411,584 bytes (12 Oct 2026, macOS, `/usr/bin/time -l`,
/// the revision written in commits of 524,288 rows), against 1,160,118,272
/// when the revision was one commit of the whole month (11 Oct 2026, this
/// bench under the same tool). The allowance sits between the two, about a
/// fifth above the first, so a writer that goes back to one commit of the
/// month fails this row, and an allocator that keeps a little more than
/// macOS's does not.
const FULL_MONTH_PEAK_ALLOWED: u64 = 640 << 20;

/// C-RP-01 — a month repair costs the same per merged bar at a month filled
/// to its grid ceiling as at a month of real size, and the month it leaves is
/// indexed. D-4875 (1), D-4898; `docs/06-limits.md` L-IA-26.
///
/// `repair::repair_month` is O(held + offered) and claims nothing less: it
/// reads the served month whole and writes a whole successor. What this row
/// holds is that it is no WORSE than linear (the cost of one merged bar at
/// 2,678,400 rows against the same at 448,317, within the gate's ceiling),
/// that a time lookup on the repaired full month costs what C-TIX-02 allows
/// any month (its `.tix` is the revision's own), and, where the host reports
/// it, that the peak resident set of the five full-month repairs stays inside
/// [`FULL_MONTH_PEAK_ALLOWED`], which is a MEASURED allowance and not a sum of
/// type sizes: L-IA-26 says what the terms are and which of them are the
/// writer's and its sort index's, not the repair's.
///
/// Five full-month repairs and fifteen session-month repairs, each of a
/// fresh copy of the held month.
fn month_repair_is_linear_and_leaves_the_month_indexed() -> bool {
    /// C-TIX-02's allowance, for the same lookup on a repaired month.
    const ALLOWED_TIX: u128 = 800;
    let session = RepairShape::session_month();
    let full = RepairShape::full_month();
    let (base, session_root) = session.measure(15);
    let _ignored = std::fs::remove_dir_all(&session_root);
    // FROM HERE THE PEAK IS THE FULL MONTH'S, where the host can restart it.
    let own_peak = restart_peak_resident();
    let (at, full_root) = full.measure(5);
    let peak = peak_resident_bytes();
    let mut ok = ratio("C-RP-01 repair_month a merged row, full month", base, at);

    // THE MONTH IT LEFT: served from the revision, full, and indexed.
    let served = match BarFile::open_existing(&full_root, repair_path(), 7) {
        Ok(f) => f,
        Err(e) => refuse(&format!("the repaired month would not open: {e}")),
    };
    if u128::from(served.header().n_valid) != full.merged()
        || served.header().n_valid != repair::row_ceiling(repair_path())
    {
        refuse("the repaired month is not full to its ceiling");
    }
    if served.time_lookup() != store::file::TimeLookup::Indexed {
        refuse("the repaired month has no ready time index; the row would time the bisection");
    }
    let stamps: Vec<i64> = (0..14i64)
        .map(|step| JULY_2024_IST_START.saturating_add((step * 206_030).saturating_mul(1_000_000)))
        .collect();
    let mut next = 0usize;
    let lookup = cost_ps(280, || {
        let ts = stamps.get(next % stamps.len()).copied().unwrap_or(0);
        next = next.wrapping_add(1);
        served.first_at_or_after(black_box(ts))
    });
    ok &= budget(
        "C-RP-01 first_at_or_after, repaired full month",
        floor_ps(served.layout()),
        lookup,
        ALLOWED_TIX,
    );
    drop(served);
    let _ignored = std::fs::remove_dir_all(&full_root);

    let allowed = FULL_MONTH_PEAK_ALLOWED;
    match peak {
        Some(peak) => {
            let held = peak <= allowed;
            println!(
                "  C-RP-01 peak resident set, full month         {peak} B against {allowed} B allowed ({})   {}",
                if own_peak {
                    "the five full-month repairs' own"
                } else {
                    "the whole process's: the host would not restart its peak"
                },
                if held { "ok" } else { "OVER BUDGET" }
            );
            ok &= held;
        }
        None => println!(
            "  C-RP-01 peak resident set, full month         NOT MEASURED on this host (no /proc/self/status); {allowed} B allowed where it is"
        ),
    }
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
    ok &= verdict_ceiling_is_refused_whole();
    ok &= verdict_lookup_is_flat_in_the_day();
    verdict_lookup_over_a_year();
    ok &= year_index_read_is_flat();
    ok &= generation_write_stays_n_log_n();
    ok &= month_repair_is_linear_and_leaves_the_month_indexed();
    ok &= safe_files::run();
    ok &= months::run();
    if ok {
        println!("all ratios within the ceiling");
    } else {
        println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
        std::process::exit(1);
    }
}
