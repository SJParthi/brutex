//! Gate 8 for the option lists `/db` draws its pickers from,
//! `/census/facets.json`: an underlying's expiries and an expiry's strikes cost
//! the same however large the census is (R3-14, RA-064).
//!
//! Map probes of the facet index, which is built once per census read and
//! moved forward by what the census appended; the build is the `cold` figure.

use super::served::{EXPIRY, Served, flat, ok_with};

/// The routes this module times, read by `.github/gates_routes.rs`.
pub(crate) const ROUTES: &[&str] = &["/census/facets.json"];

/// Every target of options at 1x, 10x and 100x, each p50 ratio gated. Run by
/// `crates/api/benches/ratio.rs`'s `main` over the stores `served` built.
pub(crate) fn lists_are_flat(stores: &mut [Served]) -> bool {
    flat("options", stores, |_served| {
        vec![
            ok_with(
                "feed root",
                "/census/facets.json?feed=gdfl".to_owned(),
                "FNO",
            ),
            ok_with(
                "an underlying's expiries",
                "/census/facets.json?feed=gdfl&segment=FNO&underlying=NIFTY".to_owned(),
                EXPIRY,
            ),
            ok_with(
                "an expiry's strikes",
                format!(
                    "/census/facets.json?feed=gdfl&segment=FNO&underlying=NIFTY&expiry={EXPIRY}"
                ),
                "1230000",
            ),
        ]
    })
}
