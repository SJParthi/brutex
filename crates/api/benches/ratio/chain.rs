//! Gate 8 for `/chain.json`: a chain of a fixed number of strikes at one
//! instant costs the same whatever the months behind it hold and however large
//! the census is (R3-14, RA-064).
//!
//! NIFTY's 2024-06-27 expiry, `served::STRIKES` strikes a side, asked at the
//! middle bar of the month (the spot found by its seek) and with `asof=prev`.

use super::served::{EXPIRY, Served, flat, ok_with, session_second};

/// The routes this module times, read by `.github/gates_routes.rs`.
pub(crate) const ROUTES: &[&str] = &["/chain.json"];

/// Every target of chain at 1x, 10x and 100x, each p50 ratio gated. Run by
/// `crates/api/benches/ratio.rs`'s `main` over the stores `served` built.
pub(crate) fn chains_are_flat(stores: &mut [Served]) -> bool {
    flat("chain", stores, |served| {
        let at = session_second(served.bars / 2);
        vec![
            ok_with(
                "one instant",
                format!(
                    "/chain.json?feed=gdfl&underlying=NIFTY&expiry={EXPIRY}&timeframe=1s&at={at}"
                ),
                "1230000",
            ),
            ok_with(
                "one instant, asof=prev",
                format!(
                    "/chain.json?feed=gdfl&underlying=NIFTY&expiry={EXPIRY}&timeframe=1s&at={at}&asof=prev"
                ),
                "1230000",
            ),
        ]
    })
}
