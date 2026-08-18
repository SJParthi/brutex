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
                ts_micros: raw.saturating_mul(60_000_000),
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

/// The path every bench file uses. One month, one symbol, one timeframe.
fn bench_path() -> StorePath<'static> {
    match StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
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

/// C-16 — reading one record costs the same whatever the file holds.
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
    ok &= ratio("C-16 read_record[0], 10x file", base, first(&medium));
    ok &= ratio("C-16 read_record[0], 100x file", base, first(&large));

    let base_last = last(&small, 1_000);
    ok &= ratio(
        "C-16 read_record[last], 10x file",
        base_last,
        last(&medium, 10_000),
    );
    ok &= ratio(
        "C-16 read_record[last], 100x file",
        base_last,
        last(&large, 100_000),
    );

    // And the two ends of the SAME file cost the same, which is the shape a
    // scan would break first.
    ok &= ratio(
        "C-16 read_record: first against last, same file",
        first(&large),
        last(&large, 100_000),
    );
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
    if ok {
        println!("all ratios within the ceiling");
    } else {
        println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
        std::process::exit(1);
    }
}
