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

/// The three sizes' min-of-trials costs, printed as a cost PER UNIT of the
/// size axis and the 1x -> 100x ratio, for a row whose own doc states it
/// grows with that axis. Never gated: the figure is the finding.
pub(crate) fn stated(label: &str, costs: &[u128]) {
    let (Some(&base), Some(&big)) = (costs.first(), costs.get(2)) else {
        refuse("three sizes measured");
    };
    let (Some(&small), Some(&large)) = (MONTH_COUNTS.first(), MONTH_COUNTS.get(2)) else {
        refuse("three sizes named");
    };
    let per_unit = big.saturating_sub(base) / u128::try_from(large - small).unwrap_or(1).max(1);
    let permille = (big * 1_000).checked_div(base).unwrap_or(0);
    println!(
        "  {label:<52} {base:>12} ps -> {big:>13} ps  {}.{:03}x  {per_unit:>10} ps per extra unit  (stated, not gated)",
        permille / 1_000,
        permille % 1_000,
    );
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
        stated(&format!("{label} (O(months))"), &costs);
    }
    let mut costs = Vec::with_capacity(roots.len());
    for (months, root) in roots {
        let middle = nth_month(months / 2);
        let asked = query(
            *months,
            None,
            Some(stamp(middle, BARS_PER_MONTH / 2)),
            SortKey::Ts,
        );
        spread(
            &format!("window at=, {months} months"),
            SPREAD_CALLS,
            || page(black_box(root), black_box(&asked)),
        );
        costs.push(cost_ps(50, || page(black_box(root), black_box(&asked))));
    }
    stated("window at= (O(months))", &costs);
}

/// `/verify.json`'s scrub over a census of 1, 10 and 100 held months.
///
/// `api::verify::vendor` opens every held entry's file and reads its header
/// and two records, so it is O(entries) by its own doc (D-1501), which also
/// says no bench timed it. PRINTED, not asserted: the scrub is one admitted
/// request at a time (WP-33c), and the figure is the per-month cost.
fn verifies(roots: &[(usize, PathBuf)]) {
    let mut costs = Vec::with_capacity(roots.len());
    for (months, root) in roots {
        let census = api::census::read_vendor(root, Vendor::Gdfl);
        let report = api::verify::vendor(root, &census);
        if !report.verified() || report.tally.agreed != u64::try_from(*months).unwrap_or(0) {
            refuse(&format!(
                "the scrub of {months} fixture months did not agree on every one: {}",
                report.say()
            ));
        }
        spread(
            &format!("verify scrub, {months} months"),
            SPREAD_CALLS,
            || {
                api::verify::vendor(black_box(root), black_box(&census))
                    .tally
                    .agreed
            },
        );
        costs.push(cost_ps(20, || {
            api::verify::vendor(black_box(root), black_box(&census))
                .tally
                .agreed
        }));
    }
    stated("verify scrub (O(entries))", &costs);
}

/// `/spot.json` at an instant of the series' LAST month, over 1, 10 and 100
/// held months.
///
/// `SpotAsk::answer_with` opens exactly the one month file the question
/// names, so the months beside it may not cost anything. ASSERTED: a ratio
/// past the ceiling here is a spot read that walks the series.
fn spots(roots: &[(usize, PathBuf)]) -> bool {
    let mut costs = Vec::with_capacity(roots.len());
    for (months, root) in roots {
        let last = nth_month(months - 1);
        let at = stamp(last, BARS_PER_MONTH / 2) / 1_000_000;
        let query = format!(
            "feed=gdfl&exchange=NSE&symbol=NIFTY&timeframe=1s&month={}-{:02}&t={at}",
            last.year(),
            last.month()
        );
        let (status, _, body) = api::server::spot::respond(root, &query);
        if status != axum::http::StatusCode::OK || !body.contains(&format!("{at}")) {
            refuse(&format!(
                "the spot read of {months} months answered {status}: {body}"
            ));
        }
        spread(
            &format!("spot.json last month, {months} months"),
            SPREAD_CALLS,
            || {
                api::server::spot::respond(black_box(root), black_box(&query))
                    .2
                    .len()
            },
        );
        costs.push(cost_ps(200, || {
            api::server::spot::respond(black_box(root), black_box(&query))
                .2
                .len()
        }));
    }
    flat("spot.json last month (asserted)", &costs)
}

/// The `/store` page over a census of 1, 10 and 100 held months.
///
/// `store_html` reads the census through `census_now` (a hit stamps every
/// vendor's manifest) and draws one bounded page. PRINTED, not asserted: the
/// census it walks is the month count here, and the figure says whether a
/// hit's cost moves with it.
fn store_pages(roots: &[(usize, PathBuf)]) {
    let Ok(today) = pull::session::Day::new(2026, 10, 16) else {
        refuse("2026-10-16 is a day");
    };
    let mut costs = Vec::with_capacity(roots.len());
    for (months, root) in roots {
        let read = api::server::Read::new(
            api::merge::Merged::default(),
            Vec::new(),
            Vec::new(),
            0,
            0,
            0,
        );
        let censuses = api::census::read_all(root);
        let site = api::server::Site::new(read, censuses, root.clone());
        let (status, html) = api::server::store_html(&site, today, 0, "feed=gdfl");
        if status != axum::http::StatusCode::OK || !html.contains("NIFTY") {
            refuse(&format!(
                "the store page over {months} months answered {status}"
            ));
        }
        spread(
            &format!("store page, {months} months"),
            SPREAD_CALLS,
            || {
                api::server::store_html(black_box(&site), today, 0, "feed=gdfl")
                    .1
                    .len()
            },
        );
        costs.push(cost_ps(50, || {
            api::server::store_html(black_box(&site), today, 0, "feed=gdfl")
                .1
                .len()
        }));
    }
    stated("store page (rows drawn = months)", &costs);
}

/// Census entries for the census-size `/store` rows: 1x, 10x and 100x.
const CENSUS_SIZES: [usize; 3] = [1_000, 10_000, 100_000];

/// A store whose GDFL census holds `n` entries: the `n`th cash symbol's
/// September 2026 one-minute month, no bar file behind any of them. The page
/// reads the census, not the bars.
fn census_store(n: usize) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-api-bench-census-page-{n}-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&root);
    if let Err(why) = std::fs::create_dir_all(root.join("manifest")) {
        refuse(&format!("the census page root would not create: {why}"));
    }
    let Ok(mut manifest) = Manifest::open(Vendor::Gdfl, &[], &[]) else {
        refuse("a genesis census");
    };
    let Ok(month) = YearMonth::new(2026, 9) else {
        refuse("September 2026");
    };
    for i in 0..n {
        let Ok(symbol) = Symbol::new(&format!("S{i:07}")) else {
            refuse("a synthetic symbol");
        };
        let entry = Entry {
            key: api::census::Series {
                contract: None,
                exchange: Exchange::Nse,
                segment: Segment::Cash,
                symbol,
                timeframe: Timeframe::MINUTE_1,
            }
            .at(month),
            rows: 375,
            first_ts_micros: 1_788_926_700_000_000,
            last_ts_micros: 1_788_949_140_000_000,
        };
        if manifest.record(entry).is_err() {
            refuse("a census entry");
        }
    }
    let path = pull::manifest::manifest_path(&root, Vendor::Gdfl);
    if let Err(why) = std::fs::write(&path, manifest.image()) {
        refuse(&format!("the census page manifest would not write: {why}"));
    }
    root
}

/// The `/store` page over a census of 1,000, 10,000 and 100,000 entries,
/// every size drawing the same full page of rows.
///
/// A warm request reads the census through `census_now`'s cache (a `stat`
/// per vendor, D-0686) and draws one page of `PAGE_ROWS`; the order is built
/// once per census read. ASSERTED: a ratio past the ceiling is a warm page
/// whose cost tracks the census.
fn store_pages_over_the_census() -> bool {
    let Ok(today) = pull::session::Day::new(2026, 10, 16) else {
        refuse("2026-10-16 is a day");
    };
    let mut costs = Vec::with_capacity(CENSUS_SIZES.len());
    let mut roots = Vec::with_capacity(CENSUS_SIZES.len());
    for n in CENSUS_SIZES {
        let root = census_store(n);
        let read = api::server::Read::new(
            api::merge::Merged::default(),
            Vec::new(),
            Vec::new(),
            0,
            0,
            0,
        );
        let site = api::server::Site::new(read, api::census::read_all(&root), root.clone());
        let started = Instant::now();
        let (status, html) = api::server::store_html(&site, today, 0, "feed=gdfl");
        let first_us = started.elapsed().as_micros();
        let drawn = html.matches("S00").count();
        if status != axum::http::StatusCode::OK || drawn == 0 {
            refuse(&format!(
                "the store page over {n} entries answered {status}, {drawn} rows"
            ));
        }
        println!(
            "  store page first request, {n:>6} entries {first_us:>10} us  (census read once; {drawn} symbol cells drawn)"
        );
        spread(
            &format!("store page warm, {n} entries"),
            SPREAD_CALLS,
            || {
                api::server::store_html(black_box(&site), today, 0, "feed=gdfl")
                    .1
                    .len()
            },
        );
        costs.push(cost_ps(50, || {
            api::server::store_html(black_box(&site), today, 0, "feed=gdfl")
                .1
                .len()
        }));
        roots.push(root);
    }
    let (Some(&base), Some(&mid), Some(&big)) = (costs.first(), costs.get(1), costs.get(2)) else {
        refuse("three sizes measured");
    };
    let ok = ratio(
        "store page warm 1000->10000 entries",
        base,
        mid,
        RATIO_CEILING_PERMILLE,
    ) & ratio(
        "store page warm 1000->100000 entries",
        base,
        big,
        RATIO_CEILING_PERMILLE,
    );
    for root in roots {
        let _ignored = std::fs::remove_dir_all(root);
    }
    ok
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
    verifies(&roots);
    let mut ok = spots(&roots);
    store_pages(&roots);
    for (_, root) in roots {
        let _ignored = std::fs::remove_dir_all(root);
    }
    ok &= store_pages_over_the_census();
    ok
}
