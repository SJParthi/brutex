//! Gate 8 for `/dayflags.json`: a page of day records costs the same whatever
//! the month behind it holds and however large the census is (R3-14, RA-064).
//!
//! One page of five days, 2024-06-03 to 2024-06-07, asked for NIFTY's index
//! and for one of its options, at every scale; each record's bar half opens
//! the month and seeks each day's first bar through its `.tix`. The ledgers
//! the route merges are absent here and answered as absent (see `served`).

use super::served::{CONTRACT, Served, flat, ok_with};

/// The routes this module times, read by `.github/gates_routes.rs`.
pub(crate) const ROUTES: &[&str] = &["/dayflags.json"];

/// Every target of dayflags at 1x, 10x and 100x, each p50 ratio gated. Run by
/// `crates/api/benches/ratio.rs`'s `main` over the stores `served` built.
pub(crate) fn days_are_flat(stores: &mut [Served]) -> bool {
    flat("dayflags", stores, |_served| {
        vec![
            ok_with(
                "index, five days",
                "/dayflags.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-06-03&to=2024-06-07"
                    .to_owned(),
                "2024-06-03",
            ),
            ok_with(
                "option, five days",
                format!(
                    "/dayflags.json?feed=gdfl&instrument=NSE-FNO-NIFTY-{CONTRACT}&from=2024-06-03&to=2024-06-07"
                ),
                "2024-06-03",
            ),
        ]
    })
}
