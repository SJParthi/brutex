//! Gate 8 for `/db`'s Coverage grid, `/census/rollup.json`: a selection's
//! months cost the same however large the census beside them is (R3-09;
//! NEW-05, RA-008, DB-G80). Added by GF-1 (18 Oct 2026) because gate 29
//! found the route in `logs::DB_READS` with no growth bench, and a `/db`
//! read may not be owed one.
//!
//! Every fixture series is in the one bench month, so each answer is one
//! month at the one-second rung at 1x, 10x and 100x: what grows is the census
//! the facet index was moved forward over, which the route must not read.

use super::served::{Served, flat, ok_with};

/// The routes this module times, read by `.github/gates_routes.rs`.
pub(crate) const ROUTES: &[&str] = &["/census/rollup.json"];

/// The month every fixture series is in, as the roll-up writes it.
const MONTH: &str = "\"month\":\"2024-06\"";

/// A segment's months and one underlying's, at 1x, 10x and 100x, each p50
/// ratio gated. Run by `crates/api/benches/ratio.rs`'s `main` over the stores
/// `served` built.
pub(crate) fn rollups_are_flat(stores: &mut [Served]) -> bool {
    flat("rollup", stores, |_served| {
        vec![
            ok_with(
                "a segment's months",
                "/census/rollup.json?feed=gdfl&segment=FNO".to_owned(),
                MONTH,
            ),
            ok_with(
                "an underlying's months",
                "/census/rollup.json?feed=gdfl&segment=FNO&underlying=NIFTY".to_owned(),
                MONTH,
            ),
        ]
    })
}
