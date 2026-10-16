//! Gate 8 rows `cloud/attack-o1-20261016` adds to `crates/store`: what a
//! reader and a writer pay at p99, not only at the minimum.
//!
//! Every other row in this bench reports the MINIMUM of [`super::TRIALS`]
//! means, so a cost paid once in every few calls — a compaction, a cold
//! block, a slow open — is discarded by construction. These rows print p50,
//! p99 and max over single calls (`super::spread`), at three sizes each:
//!
//! * **C-MO-01, open and read beside 1, 10 and 100 months.** One
//!   `BarFile::open_existing` of the series' LAST month, a `read_record` of
//!   its middle row and a `.tix` lookup, with 1, 10 and 100 month files of
//!   the same series in the one rung directory. `api` opens a handle per
//!   request, so this is the per-request floor of every bar route. ASSERTED
//!   flat (min-of-trials ratio): a reader opens its siblings by exact name
//!   and lists no directory.
//! * **C-MO-02, warm reads and lookups at 1,024 / 10,240 / 102,400 bars.**
//!   The C-28 and C-TIX-01 operations again, read through a READER handle
//!   (`open_existing`, which is what `api` holds; C-28 reads through the
//!   writer) and reported as a spread. Printed.
//! * **C-MO-03, single-bar commits on a month of 1,024 / 10,240 / 102,400
//!   bars.** [`COMMITS`] consecutive one-bar appends. The `.srt` sort index
//!   is extended by one run per commit and rewritten WHOLE when its run
//!   count reaches `sort_index::COMPACT_AT` (`sort_index_writer.rs`,
//!   `write_whole`), so one commit in about thirty pays O(n log n) in the
//!   month. Printed, never asserted: the max is the finding, and it is
//!   recorded in `reports/cloud-attack-o1-20261016.md`.

use std::path::{Path, PathBuf};
use std::time::Instant;

use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{cost_ps, ratio, refuse, spread};

/// Months in the series directory: 1x, 10x and 100x.
const MONTH_COUNTS: [usize; 3] = [1, 10, 100];

/// Bars in each C-MO-01 month: the smallest indexed month (D-4933).
const BARS_PER_MONTH: u64 = 1_024;

/// Month sizes for C-MO-02 and C-MO-03, in bars.
const MONTH_SIZES: [u64; 3] = [1_024, 10_240, 102_400];

/// Single calls per spread line: the operator's floor for a p99 is 200.
const CALLS: u32 = 300;

/// One-bar commits per C-MO-03 line: past two compactions at `COMPACT_AT`
/// = 32, so the max includes at least one whole `.srt` rewrite.
const COMMITS: u32 = 96;

/// The symbol id every fixture file is stamped with.
const ID: u32 = 7;

/// The `i`th month from January 2015.
fn nth_month(i: usize) -> YearMonth {
    let (Ok(years), Ok(month0)) = (u16::try_from(i / 12), u8::try_from(i % 12)) else {
        refuse("a fixture month index fits a year and a month");
    };
    match YearMonth::new(2015 + years, month0 + 1) {
        Ok(month) => month,
        Err(_) => refuse("a fixture month is a real month"),
    }
}

/// The one-second NIFTY INDEX path of `month`.
fn path_of(month: YearMonth) -> StorePath<'static> {
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
        Ok(path) => path,
        Err(_) => refuse("the months bench path is a legal one"),
    }
}

/// Bar `i` of `month`, one second apart from its first IST instant.
fn bar(month: YearMonth, i: u64) -> Bar {
    let raw = i64::try_from(i).unwrap_or(0);
    Bar {
        ts_micros: month
            .ist_bounds_micros()
            .0
            .saturating_add(raw.saturating_mul(1_000_000)),
        open: 2_000_000 + raw,
        high: 2_000_100 + raw,
        low: 1_999_900 + raw,
        close: 2_000_050 + raw,
        volume: 1_000,
        open_interest: 0,
    }
}

/// A fresh store root of this process's own.
fn root_of(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("brutex-bench-months-{name}-{}", std::process::id()));
    let _ignored = std::fs::remove_dir_all(&root);
    if let Err(why) = std::fs::create_dir_all(&root) {
        refuse(&format!("the months bench root would not create: {why}"));
    }
    root
}

/// Writes `month` with `n` bars in one append, refusing an unindexed month.
fn write_month(root: &Path, month: YearMonth, n: u64) -> BarFile {
    let mut file = match BarFile::open_or_create(root, path_of(month), ID) {
        Ok(file) => file,
        Err(why) => refuse(&format!("a months bench month would not open: {why}")),
    };
    let batch: Vec<Bar> = (0..n).map(|i| bar(month, i)).collect();
    if let Err(why) = file.append(&batch) {
        refuse(&format!("a months bench month would not fill: {why}"));
    }
    if file.time_lookup() != store::file::TimeLookup::Indexed {
        refuse("every months bench month is indexed (D-4933)");
    }
    file
}

/// Opens `month` for reading, reads its middle row and locates its middle
/// stamp: the per-request floor of a bar route.
fn open_and_read(root: &Path, month: YearMonth) -> u64 {
    let file = match BarFile::open_existing(root, path_of(month), ID) {
        Ok(file) => file,
        Err(why) => refuse(&format!("a months bench month would not reopen: {why}")),
    };
    let middle = BARS_PER_MONTH / 2;
    let read = file.read_record(middle).map_or(0, |bar| bar.close);
    let found = file
        .first_at_or_after(bar(month, middle).ts_micros)
        .unwrap_or(u64::MAX);
    if found != middle || read == 0 {
        refuse("the reopened month answered something other than it holds");
    }
    found
}

/// C-MO-01: open + read + lookup of the last month beside 1, 10 and 100.
fn open_is_flat_in_the_months_beside_it() -> bool {
    let mut costs = Vec::with_capacity(MONTH_COUNTS.len());
    let mut roots = Vec::with_capacity(MONTH_COUNTS.len());
    for months in MONTH_COUNTS {
        let root = root_of(&format!("open-{months}"));
        for i in 0..months {
            drop(write_month(&root, nth_month(i), BARS_PER_MONTH));
        }
        let last = nth_month(months - 1);
        spread(
            &format!("C-MO-01 open+read+tix, {months} months in the dir"),
            CALLS,
            || {
                std::hint::black_box(open_and_read(&root, last));
            },
        );
        costs.push(cost_ps(50, || open_and_read(&root, last)));
        roots.push(root);
    }
    let (Some(&base), Some(&mid), Some(&big)) = (costs.first(), costs.get(1), costs.get(2)) else {
        refuse("three sizes measured");
    };
    let ok = ratio("C-MO-01 open+read+tix 1->10 months", base, mid)
        & ratio("C-MO-01 open+read+tix 1->100 months", base, big);
    for root in roots {
        let _ignored = std::fs::remove_dir_all(root);
    }
    ok
}

/// C-MO-02: warm `read_record` and `.tix` lookup through a reader, spread.
fn warm_reads_have_a_p99() {
    let month = nth_month(0);
    for n in MONTH_SIZES {
        let root = root_of(&format!("warm-{n}"));
        drop(write_month(&root, month, n));
        let file = match BarFile::open_existing(&root, path_of(month), ID) {
            Ok(file) => file,
            Err(why) => refuse(&format!("the warm month would not reopen: {why}")),
        };
        let mut i = 0_u64;
        spread(&format!("C-MO-02 read_record, {n} bars"), CALLS, || {
            // A stride through the month, so the block cache is warm only
            // where the walk is sequential: 73 records share a block.
            i = (i + 7_919) % n;
            std::hint::black_box(file.read_record(i).map_or(0, |bar| bar.close));
        });
        let mut j = 0_u64;
        spread(
            &format!("C-MO-02 first_at_or_after (.tix), {n} bars"),
            CALLS,
            || {
                j = (j + 7_919) % n;
                std::hint::black_box(file.first_at_or_after(bar(month, j).ts_micros).unwrap_or(0));
            },
        );
        drop(file);
        let _ignored = std::fs::remove_dir_all(root);
    }
}

/// C-MO-03: [`COMMITS`] one-bar commits on a month already holding `n`.
fn single_bar_commits_have_a_p99() {
    let month = nth_month(0);
    for n in MONTH_SIZES {
        let root = root_of(&format!("append-{n}"));
        let mut file = write_month(&root, month, n);
        let mut ns: Vec<u128> = Vec::with_capacity(usize::try_from(COMMITS).unwrap_or(0));
        for next in n..n + u64::from(COMMITS) {
            let one = [bar(month, next)];
            let start = Instant::now();
            if let Err(why) = file.append(&one) {
                refuse(&format!("a one-bar commit was refused: {why}"));
            }
            ns.push(start.elapsed().as_nanos());
        }
        // THE THREE SLOWEST COMMITS BY POSITION: a compaction lands on a
        // cadence (`COMPACT_AT`), a slow device flush on none.
        let mut ranked: Vec<(u128, usize)> = ns.iter().copied().zip(1..).collect();
        ranked.sort_unstable_by(|a, b| b.cmp(a));
        let slowest: Vec<String> = ranked
            .iter()
            .take(3)
            .map(|(at, k)| format!("#{k} {} us", at / 1_000))
            .collect();
        let slowest = slowest.join(", ");
        ns.sort_unstable();
        let at = |q: usize| {
            ns.get(ns.len().saturating_sub(1) * q / 100)
                .copied()
                .unwrap_or(0)
        };
        println!(
            "  {:<56} p50 {:>9} ns  p99 {:>9} ns  max {:>10} ns  n={COMMITS}  (slowest: {slowest})",
            format!("C-MO-03 one-bar commit, month of {n} bars"),
            at(50),
            at(99),
            ns.last().copied().unwrap_or(0)
        );
        drop(file);
        let _ignored = std::fs::remove_dir_all(root);
    }
}

/// Every row. Run by `crates/store/benches/ratio.rs`'s `main`.
pub(crate) fn run() -> bool {
    let ok = open_is_flat_in_the_months_beside_it();
    warm_reads_have_a_p99();
    single_bar_commits_have_a_p99();
    ok
}
