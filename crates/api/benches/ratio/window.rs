//! Gate 8 for `/bars/window.json`'s seek: one page of bars, in time order,
//! costs the same whatever the month behind it holds.
//!
//! `api::bars::window` is the function the route calls. Timed newest first at
//! offset 0 and at the last page, over one one-second month of 1,000, 10,000
//! and 100,000 bars on disk, reading 50 bars a page: the open, the `.tix` seek
//! and the page read are all in the figure, as they are on the route.
//!
//! WP-31 owns this file from here and adds its multi-series and day-bound
//! rows to it.

use std::hint::black_box;

use api::bars::{SortKey, window};
use brutex_core::vendor::Vendor;
use store::path::Timeframe;

use super::{MONTH_SIZES, RATIO_CEILING_PERMILLE, bench_month, cost_ps, month_file, ratio, refuse};

/// Bars a page asks for: the grid's own page.
const PAGE: usize = 50;

/// One page at `offset`, newest first, from the month under `root`.
fn page(root: &std::path::Path, offset: usize) -> usize {
    match window(
        root,
        Vendor::Gdfl,
        "NSE",
        "INDEX",
        "NIFTY",
        Timeframe::SECOND_1,
        None,
        bench_month(),
        bench_month(),
        SortKey::Ts,
        true,
        offset,
        PAGE,
        false,
    ) {
        Ok(answer) if answer.faults.is_empty() => answer.bars.len(),
        Ok(answer) => refuse(&format!("the window read faults: {:?}", answer.faults)),
        Err(why) => refuse(&format!("the window refused: {why}")),
    }
}

/// The seek at offset 0 and at the last page, at every size, and whether both
/// stayed flat between 1x and 100x. Run by `crates/api/benches/ratio.rs`'s
/// `main`.
pub(crate) fn seeks_are_flat() -> bool {
    let months: Vec<(usize, std::path::PathBuf)> = MONTH_SIZES
        .iter()
        .map(|&n| {
            let (file, root) = month_file("window", n);
            drop(file);
            (n, root)
        })
        .collect();
    let mut ok = true;
    for (label, at) in [("first page", 0usize), ("last page", 1)] {
        let mut costs = Vec::with_capacity(months.len());
        for (n, root) in &months {
            let offset = if at == 0 { 0 } else { n.saturating_sub(PAGE) };
            if page(root, offset) != PAGE {
                refuse("a page of the fixture month is a full page");
            }
            costs.push(cost_ps(200, || page(black_box(root), black_box(offset))));
        }
        let (Some(&base), Some(&mid), Some(&big)) = (costs.first(), costs.get(1), costs.get(2))
        else {
            refuse("three sizes measured");
        };
        ok &= ratio(
            &format!("window seek {label} {}->{}", MONTH_SIZES[0], MONTH_SIZES[1]),
            base,
            mid,
            RATIO_CEILING_PERMILLE,
        );
        ok &= ratio(
            &format!("window seek {label} {}->{}", MONTH_SIZES[0], MONTH_SIZES[2]),
            base,
            big,
            RATIO_CEILING_PERMILLE,
        );
    }
    for (_, root) in months {
        let _ignored = std::fs::remove_dir_all(root);
    }
    ok
}
