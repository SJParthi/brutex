//! Gate 8 for the routes whose cost may track how many MONTHS a series
//! holds, not how many bars one month holds (`cloud/attack-o1-20261016`).
//!
//! # Why a second size axis
//!
//! Every other row under `benches/ratio/` grows ONE month: 1,024, 10,240 and
//! 102,400 bars (`MONTH_SIZES`). That proves a seek inside a month is flat in
//! the month. It cannot see a cost paid once per month of the range — a
//! `stat`, a pointer open, a prefix sum — because every one of those fixtures
//! holds exactly one month. This file holds the bars per month FIXED at
//! [`BARS_PER_MONTH`] and grows the series instead: 1, 10 and 100 months of
//! the same one-second NIFTY INDEX series, every month indexed (`.tix` and
//! `.srt`, D-4933), with the vendor census naming every month.
//!
//! # What is printed
//!
//! For every row: p50, p99 and max over single calls (nanoseconds), at each
//! of the three sizes, and the 1x -> 10x and 1x -> 100x ratios of the
//! min-of-trials figure the other rows use. A row is ASSERTED only where the
//! code claims the cost does not track the months (a page read through the
//! global prefix, the `at=` index of one instant): a ratio past
//! [`RATIO_CEILING_PERMILLE`] there is a breach. A row whose cost is stated
//! as O(months) by its own doc is printed, never asserted, and the report
//! `reports/cloud-attack-o1-20261016.md` records its numbers against the
//! `docs/06-limits.md` paragraph that names it.
//!
//! Run by `crates/api/benches/ratio.rs`'s `main`.

use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Instant;

use api::bars::{Filter, SeriesKey, SortKey, WindowQuery, window};
use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use pull::manifest::{Entry, Manifest};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{RATIO_CEILING_PERMILLE, cost_ps, ratio, refuse};

/// Months per series: 1x, 10x and 100x.
pub(crate) const MONTH_COUNTS: [usize; 3] = [1, 10, 100];

/// Bars in every month: the smallest month the store indexes
/// (`store::time_index::INDEX_MIN_ROWS`, D-4933), so every month takes the
/// indexed path at every size and the only thing that grows is the count.
const BARS_PER_MONTH: usize = 1_024;

/// Bars a page asks for: the grid's own page.
const PAGE: usize = 50;

/// Single calls per p50/p99/max line. 200 is the operator's floor for a p99.
const SPREAD_CALLS: usize = 300;

/// The first month of every fixture series. Old enough that 100 months end
/// before the bench's other fixtures' June 2024.
const FIRST_YEAR: u16 = 2015;

/// The `i`th month of the fixture series, from January [`FIRST_YEAR`].
fn nth_month(i: usize) -> YearMonth {
    let (Ok(year_offset), Ok(month0)) = (u16::try_from(i / 12), u8::try_from(i % 12)) else {
        refuse("a fixture month index fits a year and a month");
    };
    match YearMonth::new(FIRST_YEAR + year_offset, month0 + 1) {
        Ok(month) => month,
        Err(_) => refuse("a fixture month is a real month"),
    }
}

/// The stamp of bar `i` of `month`: one second apart from the month's first
/// IST instant.
fn stamp(month: YearMonth, i: usize) -> i64 {
    let raw = i64::try_from(i).unwrap_or(i64::MAX);
    month
        .ist_bounds_micros()
        .0
        .saturating_add(raw.saturating_mul(1_000_000))
}

/// The low 32 bits of the symbol's FNV-1a: the id `pull::ingest` stamps and
/// every reader checks on open.
fn symbol_id(symbol: &str) -> u32 {
    match u32::try_from(brutex_core::universe::fnv1a(symbol) & u64::from(u32::MAX)) {
        Ok(id) => id,
        Err(_) => refuse("the low 32 bits of a hash fit 32 bits"),
    }
}

/// Writes one indexed NIFTY INDEX one-second month of [`BARS_PER_MONTH`] bars
/// under `root`, refusing a month the store gave no `.tix` or no `.srt`.
fn write_month(root: &Path, month: YearMonth, base: i64) {
    let Ok(path) = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month,
        file: FileKind::Bars,
    }) else {
        refuse("the months bench path is a legal one")
    };
    let mut file = match store::file::BarFile::open_or_create(root, path, symbol_id("NIFTY")) {
        Ok(file) => file,
        Err(why) => refuse(&format!("a months bench month would not open: {why}")),
    };
    let batch: Vec<Bar> = (0..BARS_PER_MONTH)
        .map(|i| {
            let raw = i64::try_from(i).unwrap_or(0);
            Bar {
                ts_micros: stamp(month, i),
                open: base + raw,
                high: base + 100 + raw,
                low: base - 100 + raw,
                close: base + 50 + raw,
                volume: 1_000,
                open_interest: OI_NULL,
            }
        })
        .collect();
    if let Err(why) = file.append(&batch) {
        refuse(&format!("a months bench month would not fill: {why}"));
    }
    let srt = path.with_file(FileKind::SortIndex).to_path_buf(root);
    if file.time_lookup() != store::file::TimeLookup::Indexed || !srt.is_file() {
        refuse("every months bench month is indexed (.tix and .srt), D-4933");
    }
}

/// The census entry of one fixture month: what `pull::ingest` records for it.
fn entry_of(month: YearMonth) -> Entry {
    let Ok(symbol) = Symbol::new("NIFTY") else {
        refuse("NIFTY is a symbol");
    };
    Entry {
        key: api::census::Series {
            contract: None,
            exchange: Exchange::Nse,
            segment: Segment::Index,
            symbol,
            timeframe: Timeframe::SECOND_1,
        }
        .at(month),
        rows: u64::try_from(BARS_PER_MONTH).unwrap_or(u64::MAX),
        first_ts_micros: stamp(month, 0),
        last_ts_micros: stamp(month, BARS_PER_MONTH - 1),
    }
}

/// A store of `months` fixture months and the GDFL census naming each one.
pub(crate) fn store_of(months: usize) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-api-bench-months-{months}-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&root);
    if let Err(why) = std::fs::create_dir_all(root.join("manifest")) {
        refuse(&format!("the months bench root would not create: {why}"));
    }
    let Ok(mut manifest) = Manifest::open(Vendor::Gdfl, &[], &[]) else {
        refuse("a genesis census");
    };
    for i in 0..months {
        let month = nth_month(i);
        // EACH MONTH'S PRICES START HIGHER THAN THE LAST, so a value sort's
        // first and last pages are in the first and last months.
        let base = 1_000_000 + i64::try_from(i).unwrap_or(0) * 10_000;
        write_month(&root, month, base);
        if manifest.record(entry_of(month)).is_err() {
            refuse("a census entry");
        }
    }
    let path = pull::manifest::manifest_path(&root, Vendor::Gdfl);
    if let Err(why) = std::fs::write(&path, manifest.image()) {
        refuse(&format!("the months bench census would not write: {why}"));
    }
    root
}

/// Single calls of `op`, sorted: p50, p99 and max in nanoseconds, printed
/// and returned. The p99 is the sample at rank `ceil(0.99 n)`.
pub(crate) fn spread<T>(label: &str, calls: usize, mut op: impl FnMut() -> T) -> [u128; 3] {
    let mut ns: Vec<u128> = (0..calls)
        .map(|_| {
            let start = Instant::now();
            black_box(op());
            start.elapsed().as_nanos()
        })
        .collect();
    ns.sort_unstable();
    let rank = |permille: usize| {
        let at = (ns.len() * permille).div_ceil(1_000).saturating_sub(1);
        ns.get(at).copied().unwrap_or(0)
    };
    let out = [rank(500), rank(990), ns.last().copied().unwrap_or(0)];
    println!(
        "  {label:<52} p50 {:>9} ns  p99 {:>9} ns  max {:>10} ns  n={calls}",
        out[0], out[1], out[2]
    );
    out
}

/// The three sizes' min-of-trials costs, 1x -> 10x and 1x -> 100x, printed;
/// returns whether both stayed under the ceiling.
pub(crate) fn flat(label: &str, costs: &[u128]) -> bool {
    let (Some(&base), Some(&mid), Some(&big)) = (costs.first(), costs.get(1), costs.get(2)) else {
        refuse("three sizes measured");
    };
    ratio(
        &format!("{label} {}->{} months", MONTH_COUNTS[0], MONTH_COUNTS[1]),
        base,
        mid,
        RATIO_CEILING_PERMILLE,
    ) & ratio(
        &format!("{label} {}->{} months", MONTH_COUNTS[0], MONTH_COUNTS[2]),
        base,
        big,
        RATIO_CEILING_PERMILLE,
    )
}

/// A window over every month of a `months`-month fixture series.
fn query(months: usize, offset: Option<usize>, at: Option<i64>, sort: SortKey) -> WindowQuery {
    WindowQuery {
        vendor: Vendor::Gdfl,
        timeframe: Timeframe::SECOND_1,
        series: vec![SeriesKey::new("NSE", "INDEX", "NIFTY", None)],
        from: nth_month(0),
        to: nth_month(months - 1),
        sort,
        desc: true,
        offset,
        limit: PAGE,
        want_extremes: false,
        at,
        filter: Filter::default(),
        lookback: true,
        coverage: false,
    }
}

/// One page's row count, or a refusal.
fn page(root: &Path, asked: &WindowQuery) -> usize {
    match window(root, asked) {
        Ok(answer) if answer.faults.is_empty() => answer.bars.len(),
        Ok(answer) => refuse(&format!("the window read faults: {:?}", answer.faults)),
        Err(why) => refuse(&format!("the window refused: {why}")),
    }
}

/// `/bars/window.json` over a range of 1, 10 and 100 held months.
///
/// `api::bars::window`'s own doc states the time-ordered seek as "`O(months)`
/// `stat`s plus `O(limit)` record reads", and `SeriesPlan::build` admits each
/// held month of the range with one generation-pointer open and one header
/// `stat` (`crates/api/src/bars_series.rs`, `admit`). So these rows are
/// PRINTED, not asserted: the month count is the size axis by design, capped
/// by `MAX_WINDOW_MONTHS` (240), and the numbers say what one month costs.
fn windows(roots: &[(usize, PathBuf)]) {
    for (label, last, sort) in [
        ("window seek first page", false, SortKey::Ts),
        ("window seek last page", true, SortKey::Ts),
        ("window sort=c first page", false, SortKey::Close),
    ] {
        let mut costs = Vec::with_capacity(roots.len());
        for (months, root) in roots {
            let rows = months * BARS_PER_MONTH;
            let offset = if last { rows.saturating_sub(PAGE) } else { 0 };
            let asked = query(*months, Some(offset), None, sort);
            if page(root, &asked) != PAGE {
                refuse("a page of the months fixture is a full page");
            }
            spread(&format!("{label}, {months} months"), SPREAD_CALLS, || {
                page(black_box(root), black_box(&asked))
            });
            costs.push(cost_ps(50, || page(black_box(root), black_box(&asked))));
        }
        // PRINTED, NOT ASSERTED: O(months) by the function's own doc.
        let _stated = flat(&format!("{label} (O(months), unasserted)"), &costs);
    }
    let mut costs = Vec::with_capacity(roots.len());
    for (months, root) in roots {
        let middle = nth_month(months / 2);
        let asked = query(*months, None, Some(stamp(middle, BARS_PER_MONTH / 2)), SortKey::Ts);
        spread(&format!("window at=, {months} months"), SPREAD_CALLS, || {
            page(black_box(root), black_box(&asked))
        });
        costs.push(cost_ps(50, || page(black_box(root), black_box(&asked))));
    }
    let _stated = flat("window at= (O(months), unasserted)", &costs);
}

/// Every month-axis row. Run by `crates/api/benches/ratio.rs`'s `main`.
pub(crate) fn rows_are_measured() -> bool {
    let started = Instant::now();
    let roots: Vec<(usize, PathBuf)> = MONTH_COUNTS.iter().map(|&m| (m, store_of(m))).collect();
    println!(
        "  months fixture: 1, 10 and 100 months of {BARS_PER_MONTH} indexed seconds, built in {} ms",
        started.elapsed().as_millis()
    );
    windows(&roots);
    for (_, root) in roots {
        let _ignored = std::fs::remove_dir_all(root);
    }
    true
}
