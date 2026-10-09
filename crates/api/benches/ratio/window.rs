//! Gate 8 for `/bars/window.json`: one page of bars costs the same whatever
//! the months behind it hold — in time order over one series, merged by time
//! over two, and for the global index of an instant (`at=`). D-3326.
//!
//! `api::bars::window` is the function the route calls. Timed newest first at
//! offset 0 and at the last page, over one-second months of 1,000, 10,000 and
//! 100,000 bars on disk, reading 50 bars a page: the `stat`s, the `.tix` seek
//! and the page read are all in the figure, as they are on the route.
//!
//! **What is not here, said so.** A non-time sort streams every kept row and a
//! time-of-day bound locates two instants per day: both grow with the window
//! by design (D-3326 states them), so a flatness ratio would assert the wrong
//! thing. Their costs are printed beside the ratios, unasserted, and their
//! p50/p99 on real data are in the WP-31 report for `docs/06-limits.md`.

use std::hint::black_box;
use std::time::Instant;

use api::bars::{Filter, SeriesKey, SortKey, WindowQuery, window};
use brutex_core::vendor::Vendor;
use store::path::Timeframe;

use super::{
    MONTH_SIZES, RATIO_CEILING_PERMILLE, bench_month, cost_ps, month_file, ratio, refuse, second,
};

/// Bars a page asks for: the grid's own page.
const PAGE: usize = 50;

/// The second series every month gets beside `month_file`'s NIFTY: the same
/// instants, so every one of them is a tie the merge orders by series.
const SECOND: &str = "BANKNIFTY";

/// The query over `symbols` (INDEX, GDFL, one second, the bench month).
fn query(symbols: &[&str], offset: Option<usize>, at: Option<i64>, filter: Filter) -> WindowQuery {
    WindowQuery {
        vendor: Vendor::Gdfl,
        timeframe: Timeframe::SECOND_1,
        series: symbols
            .iter()
            .map(|symbol| SeriesKey::new("NSE", "INDEX", symbol, None))
            .collect(),
        from: bench_month(),
        to: bench_month(),
        sort: SortKey::Ts,
        desc: true,
        offset,
        limit: PAGE,
        want_extremes: false,
        at,
        filter,
        lookback: true,
        coverage: false,
    }
}

/// One page, or the index of an instant, from the months under `root`.
fn page(root: &std::path::Path, asked: &WindowQuery) -> usize {
    match window(root, asked) {
        Ok(answer) if answer.faults.is_empty() => answer.bars.len(),
        Ok(answer) => refuse(&format!("the window read faults: {:?}", answer.faults)),
        Err(why) => refuse(&format!("the window refused: {why}")),
    }
}

/// Appends a BANKNIFTY month of `n` bars at `month_file`'s instants.
fn second_series(root: &std::path::Path, n: usize) {
    let Ok(path) = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: SECOND,
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: bench_month(),
        file: store::path::FileKind::Bars,
    }) else {
        refuse("the second bench path is a legal one")
    };
    let Ok(symbol_id) = u32::try_from(brutex_core::universe::fnv1a(SECOND) & u64::from(u32::MAX))
    else {
        refuse("the low 32 bits of a hash fit 32 bits")
    };
    let mut file = match store::file::BarFile::open_or_create(root, path, symbol_id) {
        Ok(file) => file,
        Err(why) => refuse(&format!("the second bench month would not open: {why}")),
    };
    let batch: Vec<store::format::Bar> = (0..n)
        .map(|i| {
            let raw = i64::try_from(i).unwrap_or(0);
            store::format::Bar {
                ts_micros: second(i),
                open: 4_000_000 + raw,
                high: 4_000_100 + raw,
                low: 3_999_900 + raw,
                close: 4_000_050 + raw,
                volume: 0,
                open_interest: store::format::OI_NULL,
            }
        })
        .collect();
    if let Err(why) = file.append(&batch) {
        refuse(&format!("the second bench month would not fill: {why}"));
    }
}

/// The three sizes' costs of `op`, and whether 1x -> 10x and 1x -> 100x stay
/// under the ceiling.
fn flat(label: &str, costs: &[u128]) -> bool {
    let (Some(&base), Some(&mid), Some(&big)) = (costs.first(), costs.get(1), costs.get(2)) else {
        refuse("three sizes measured");
    };
    ratio(
        &format!("{label} {}->{}", MONTH_SIZES[0], MONTH_SIZES[1]),
        base,
        mid,
        RATIO_CEILING_PERMILLE,
    ) & ratio(
        &format!("{label} {}->{}", MONTH_SIZES[0], MONTH_SIZES[2]),
        base,
        big,
        RATIO_CEILING_PERMILLE,
    )
}

/// The seek at offset 0 and at the last page, the two-series merge at both,
/// and `at=`, at every size, and whether each stayed flat between 1x and
/// 100x. Run by `crates/api/benches/ratio.rs`'s `main`.
pub(crate) fn seeks_are_flat() -> bool {
    let months: Vec<(usize, std::path::PathBuf)> = MONTH_SIZES
        .iter()
        .map(|&n| {
            let (file, root) = month_file("window", n);
            drop(file);
            second_series(&root, n);
            (n, root)
        })
        .collect();
    let mut ok = true;
    for (label, symbols) in [
        ("window seek", &["NIFTY"][..]),
        ("window merge k=2", &["NIFTY", SECOND][..]),
    ] {
        for (place, last) in [("first page", false), ("last page", true)] {
            let mut costs = Vec::with_capacity(months.len());
            for (n, root) in &months {
                let rows = n * symbols.len();
                let offset = if last { rows.saturating_sub(PAGE) } else { 0 };
                let asked = query(symbols, Some(offset), None, Filter::default());
                if page(root, &asked) != PAGE {
                    refuse("a page of the fixture months is a full page");
                }
                costs.push(cost_ps(200, || page(black_box(root), black_box(&asked))));
            }
            ok &= flat(&format!("{label} {place}"), &costs);
        }
    }
    let mut costs = Vec::with_capacity(months.len());
    for (n, root) in &months {
        let asked = query(
            &["NIFTY", SECOND],
            None,
            Some(second(n / 2)),
            Filter::default(),
        );
        costs.push(cost_ps(200, || page(black_box(root), black_box(&asked))));
    }
    ok &= flat("window at= k=2", &costs);
    // PRINTED, NOT ASSERTED: a time-of-day bound locates each day's window,
    // two lookups per calendar day (D-3326, O(days)), and a close sort streams
    // every row (O(bars)). Both are the stated cost.
    for (n, root) in &months {
        let tod = Filter {
            from: None,
            to: None,
            tod: Some((9 * 3_600, 9 * 3_600 + 300)),
        };
        let asked = query(&["NIFTY", SECOND], Some(0), None, tod);
        let started = Instant::now();
        let _rows = page(root, &asked);
        let tod_us = started.elapsed().as_micros();
        let mut sorted = query(&["NIFTY", SECOND], Some(0), None, Filter::default());
        sorted.sort = SortKey::Close;
        let started = Instant::now();
        let _rows = page(root, &sorted);
        println!(
            "  window tod k=2 n={n:<7} {tod_us:>8} us   sort=c k=2 {:>8} us   (stated O(days) and O(bars), not asserted)",
            started.elapsed().as_micros()
        );
    }
    for (_, root) in months {
        let _ignored = std::fs::remove_dir_all(root);
    }
    ok
}
