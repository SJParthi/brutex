//! The o1p99 attack on `crates/store` (cloud lane `cloud/o1p99-attack-20261110`).
//!
//! `benches/ratio.rs` times the bar lookups as the minimum of a few trials at
//! 1x, 10x and 100x of 1,000 records (1,024 for the time index), which keeps
//! the fastest sample and starts above the small-month threshold. This file
//! re-times the same public calls per call, prints p50 and p99, and reads
//! them at 1, 10^2, 1,023 and 1,024 (either side of
//! `time_index::INDEX_MIN_ROWS`), 10^4 and 10^6 one-second bars. A row fails
//! when its quietest-round p99 at ANY size exceeds the smallest size's by more
//! than 3.0x (the shared-CI ceiling). See `o1p99/harness.rs` and
//! `O1-ATTACK-REPORT.md` at the repository root.
//!
//! * `O1P99-STORE-01` `BarFile::read_record`, the same index again (warm).
//! * `O1P99-STORE-02` `BarFile::read_record` at a uniformly random index, so
//!   nearly every read verifies a block other than the last one (cold). From
//!   10^2 bars, the smallest file with two blocks; one block is always warm.
//! * `O1P99-STORE-03` `BarFile::first_at_or_after` (time to row, C-TIX-01's
//!   claim). Printed at every size with a random stamp, where months under
//!   1,024 bars bisect and grow (finding O1P99-F-STORE-01, reproduced by
//!   `tests/o1p99_findings.rs`); GATED from 1,024 bars to 10^6 over 64
//!   stamps spread across the month.
//! * `O1P99-STORE-04` `BarFile::open_existing` then one `first_at_or_after`:
//!   a fresh read handle per sample, the path a reader that does not keep its
//!   handle pays.
//! * `O1P99-STORE-05` `BarFile::append` of ONE bar onto a month already
//!   holding n. PRINTED, NOT GATED: it grows, because every 32nd append
//!   rewrites the `.srt` sort index whole (finding O1P99-F-STORE-02,
//!   reproduced by `tests/o1p99_findings.rs`). C-07 in `ratio` still gates
//!   the block seal step alone.
//!
//! Every file is under the system temporary directory, so the numbers are
//! that disk's and the OS page cache is warm (each file was just written).
//! A disk-cold read is not measured: dropping the page cache needs a
//! privilege this bench does not take.

use std::hint::black_box;
use std::path::PathBuf;

#[allow(
    dead_code,
    reason = "the harness is copied into every crate's o1p99 bench and not every bench calls every helper"
)]
#[path = "o1p99/harness.rs"]
mod harness;

use brutex_core::vendor::Vendor;
use harness::{FACTOR_PERMILLE, Rng, measure, verdict};
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// The sizes the read rows are read at: both sides of the 1,024-bar index
/// threshold, and the extremes.
const SIZES: [u64; 6] = [1, 100, 1_023, 1_024, 10_000, 1_000_000];

/// 2024-06-01 00:00 IST in epoch microseconds, where every bench bar starts.
const JUNE_2024_IST_START: i64 = 1_717_180_200_000_000;

/// The symbol id every bench file is created with.
const SYMBOL_ID: u32 = 7;

/// A setup failure this bench cannot measure past.
fn refuse(why: &str) -> ! {
    println!("BENCH SETUP FAILED — {why}");
    std::process::exit(1)
}

/// The one-second June 2024 NIFTY month every file here is.
fn bench_path() -> StorePath<'static> {
    let Ok(month) = YearMonth::new(2024, 6) else {
        refuse("June 2024 is a real month")
    };
    match StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month,
        file: FileKind::Bars,
    }) {
        Ok(p) => p,
        Err(_) => refuse("the bench path is a legal one"),
    }
}

/// The bar at second `i` of the month.
fn bar(i: u64) -> Bar {
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
}

/// A fresh store root holding one month of `n` one-second bars, written in
/// one batch, with the writer handle returned and the root to remove.
fn loaded(name: &str, n: u64) -> (BarFile, PathBuf) {
    let root = std::env::temp_dir().join(format!("brutex-o1p99-{name}-{n}-{}", std::process::id()));
    let _ignored = std::fs::remove_dir_all(&root);
    if let Err(e) = std::fs::create_dir_all(&root) {
        refuse(&format!("the bench root would not create: {e}"));
    }
    let mut file = match BarFile::open_or_create(&root, bench_path(), SYMBOL_ID) {
        Ok(f) => f,
        Err(e) => refuse(&format!("the bench file would not open: {e}")),
    };
    let batch: Vec<Bar> = (0..n).map(bar).collect();
    if let Err(e) = file.append(&batch) {
        refuse(&format!("the bench file would not fill: {e}"));
    }
    (file, root)
}

/// The same month opened for reading only, as a reader opens it.
fn reader(root: &std::path::Path) -> BarFile {
    match BarFile::open_existing(root, bench_path(), SYMBOL_ID) {
        Ok(f) => f,
        Err(e) => refuse(&format!("the bench month would not reopen: {e}")),
    }
}

/// Months of every size in [`SIZES`], each reopened read-only (the writer
/// handle is dropped so a reader decides its own index state).
fn months(name: &str) -> Vec<(BarFile, PathBuf, u64, Rng)> {
    SIZES
        .iter()
        .map(|&n| {
            let (writer, root) = loaded(name, n);
            drop(writer);
            (reader(&root), root, n, Rng::new(n ^ 0xb4))
        })
        .collect()
}

/// A uniformly random stamp from the month's first bar to its last.
fn random_stamp(r: &mut Rng, n: u64) -> i64 {
    let s = i64::try_from(r.below(n.max(1))).unwrap_or(0);
    JUNE_2024_IST_START.saturating_add(s.saturating_mul(1_000_000))
}

/// `O1P99-STORE-01`.
fn warm_read_is_flat(clock: u128, files: &mut [(BarFile, PathBuf, u64, Rng)]) -> bool {
    let tails = measure(
        files,
        5,
        2_000,
        1,
        clock,
        |_| {},
        |(f, _, n, _), _| {
            f.read_record(black_box(n.saturating_sub(1)))
                .map_or(0, |b| b.close.unsigned_abs())
        },
    );
    verdict(
        "O1P99-STORE-01",
        "read_record warm (same index) by bars",
        &SIZES,
        &tails,
        FACTOR_PERMILLE,
    )
}

/// `O1P99-STORE-02`.
fn random_read_is_flat(clock: u128, files: &mut [(BarFile, PathBuf, u64, Rng)]) -> bool {
    // One block holds every record of the 1-bar month, so it starts at 10^2.
    let Some(from_two_blocks) = files.get_mut(1..) else {
        refuse("the sizes start at 1 and 100")
    };
    let sizes = SIZES.get(1..).unwrap_or(&[]);
    let tails = measure(
        from_two_blocks,
        5,
        2_000,
        1,
        clock,
        |_| {},
        |(f, _, n, r), _| {
            let at = r.below(*n);
            f.read_record(black_box(at))
                .map_or(0, |b| b.close.unsigned_abs())
        },
    );
    verdict(
        "O1P99-STORE-02",
        "read_record random index (cold) by bars",
        sizes,
        &tails,
        FACTOR_PERMILLE,
    )
}

/// `O1P99-STORE-03`.
fn time_to_row_is_flat(clock: u128, files: &mut [(BarFile, PathBuf, u64, Rng)]) -> bool {
    for (f, _, n, _) in files.iter() {
        println!(
            "O1P99|O1P99-STORE-03|n={n}|lookup path {:?}",
            f.time_lookup()
        );
    }
    // Every size, a random stamp each call. PRINTED, NOT GATED: below 1,024
    // bars the month bisects (D-4933) and this row's peak is finding
    // O1P99-F-STORE-01, reproduced by `tests/o1p99_findings.rs`; above it a
    // random stamp lands on a different `.tix` page each call, which is the
    // page cache's growth and not the lookup's.
    let random = measure(
        files,
        5,
        2_000,
        1,
        clock,
        |_| {},
        |(f, _, n, r), _| {
            let ts = random_stamp(r, *n);
            f.first_at_or_after(black_box(ts)).unwrap_or(u64::MAX)
        },
    );
    let _printed = verdict(
        "O1P99-STORE-03",
        "first_at_or_after random stamp by bars (printed; see O1P99-F-STORE-01)",
        &SIZES,
        &random,
        u128::MAX,
    );
    // GATED: the indexed months only (1,024 bars and up), 64 stamps spread
    // from the first bar to the last and walked in turn, the shape C-TIX-01
    // uses with fourteen.
    let Some(indexed) = files.get_mut(3..) else {
        refuse("the sizes reach 1,024")
    };
    let sizes = SIZES.get(3..).unwrap_or(&[]);
    let tails = measure(
        indexed,
        5,
        2_000,
        1,
        clock,
        |_| {},
        |(f, _, n, _), i| {
            let step = i % 64;
            let row = step.saturating_mul(n.saturating_sub(1)) / 63;
            let s = i64::try_from(row).unwrap_or(0);
            let ts = JUNE_2024_IST_START.saturating_add(s.saturating_mul(1_000_000));
            f.first_at_or_after(black_box(ts)).unwrap_or(u64::MAX)
        },
    );
    verdict(
        "O1P99-STORE-03",
        "first_at_or_after, 64 spread stamps, indexed months by bars",
        sizes,
        &tails,
        FACTOR_PERMILLE,
    )
}

/// `O1P99-STORE-04`.
fn fresh_handle_lookup_is_flat(clock: u128, files: &mut [(BarFile, PathBuf, u64, Rng)]) -> bool {
    let tails = measure(
        files,
        4,
        300,
        1,
        clock,
        |_| {},
        |(_, root, n, r), _| {
            let ts = random_stamp(r, *n);
            match BarFile::open_existing(root, bench_path(), SYMBOL_ID) {
                Ok(f) => f.first_at_or_after(black_box(ts)).unwrap_or(u64::MAX),
                Err(_) => u64::MAX,
            }
        },
    );
    verdict(
        "O1P99-STORE-04",
        "open_existing + first_at_or_after by bars",
        &SIZES,
        &tails,
        FACTOR_PERMILLE,
    )
}

/// `O1P99-STORE-05`.
fn one_bar_append_is_flat() -> bool {
    const APPEND_SIZES: [u64; 4] = [1, 100, 10_000, 1_000_000];
    let clock = harness::clock_ps();
    let mut writers: Vec<(BarFile, PathBuf, u64)> = APPEND_SIZES
        .iter()
        .map(|&n| {
            let (w, root) = loaded("append", n);
            (w, root, n)
        })
        .collect();
    // 4 rounds x 100 appends: each sample appends the next second's bar, so a
    // month grows by 400 over the row, far under the 10^6 it starts from and
    // a visible share only of the 1-bar month, which is the row's base.
    let tails = measure(
        &mut writers,
        4,
        100,
        1,
        clock,
        |_| {},
        |(w, _, next), _| {
            let b = bar(*next);
            *next = next.saturating_add(1);
            u64::from(w.append(black_box(&[b])).is_ok())
        },
    );
    // PRINTED, NOT GATED: this row grows (every 32nd append rewrites the
    // `.srt` whole), which is finding O1P99-F-STORE-02, reproduced by
    // `tests/o1p99_findings.rs`. C-07 still gates the seal step in `ratio`.
    let ok = verdict(
        "O1P99-STORE-05",
        "append one bar by bars held (printed; see O1P99-F-STORE-02)",
        &APPEND_SIZES,
        &tails,
        u128::MAX,
    );
    for (w, root, _) in writers {
        drop(w);
        let _ignored = std::fs::remove_dir_all(root);
    }
    ok
}

fn main() {
    let clock = harness::clock_ps();
    println!(
        "o1p99 attack — crates/store, clock {} ns a pair",
        harness::ns(clock)
    );
    let mut files = months("read");
    let mut ok = true;
    ok &= warm_read_is_flat(clock, &mut files);
    ok &= random_read_is_flat(clock, &mut files);
    ok &= time_to_row_is_flat(clock, &mut files);
    ok &= fresh_handle_lookup_is_flat(clock, &mut files);
    for (f, root, _, _) in files {
        drop(f);
        let _ignored = std::fs::remove_dir_all(root);
    }
    ok &= one_bar_append_is_flat();
    if ok {
        println!("every gated o1p99 row held its ceiling");
    } else {
        println!("AN o1p99 ROW GREW PAST ITS CEILING — see the VERDICT lines marked GROWS.");
        std::process::exit(1);
    }
}
