//! Gate 8 for `/provenance.json`: where a month's bars came from, and one
//! bar's own, cost the same whatever the month holds and however large the
//! census is (R3-14, RA-064).

use super::served::{CONTRACT, Served, flat, ok_with, session_second};

/// The routes this module times, read by `.github/gates_routes.rs`.
pub(crate) const ROUTES: &[&str] = &["/provenance.json"];

/// Every target of provenance at 1x, 10x and 100x, each p50 ratio gated. Run by
/// `crates/api/benches/ratio.rs`'s `main` over the stores `served` built.
pub(crate) fn provenance_is_flat(stores: &mut [Served]) -> bool {
    flat("provenance", stores, |served| {
        let t = session_second(served.bars / 2);
        vec![
            ok_with(
                "index month",
                "/provenance.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&timeframe=1s&month=2024-06"
                    .to_owned(),
                "NIFTY",
            ),
            ok_with(
                "index bar at t",
                format!(
                    "/provenance.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&timeframe=1s&month=2024-06&t={t}"
                ),
                "NIFTY",
            ),
            ok_with(
                "option month",
                format!(
                    "/provenance.json?feed=gdfl&instrument=NSE-FNO-NIFTY-{CONTRACT}&timeframe=1s&month=2024-06"
                ),
                "NIFTY",
            ),
        ]
    })
}
