//! Gate 8 for `/bars/export.csv`, the whole bar query streamed as CSV
//! (R3-10): one export of the same rows costs the same however large the
//! month around them is.
//!
//! Every served store holds NIFTY's one-second index month at 1x, 10x and
//! 100x bars; each target exports the first 1,000 seconds of its first
//! session (`from_day` = `to_day`, `tod_to` = 09:31:39), so the rows written
//! are the same at every size and what grows is the month the reader must not
//! read past them. A time order and a value order are both timed: the first
//! is the seek path, the second the scan the window route admits.

use super::served::{Served, flat, ok_with};

/// The routes this module times, read by `.github/gates_routes.rs`.
pub(crate) const ROUTES: &[&str] = &["/bars/export.csv"];

/// The query every target exports: 1,000 one-second bars of 2024-06-03.
const QUERY: &str = "feed=gdfl&timeframe=1s&exchange=NSE&segment=INDEX&symbol=NIFTY\
                     &from=2024-06&to=2024-06&from_day=2024-06-03&to_day=2024-06-03\
                     &tod_to=09:31:39";

/// Both orders at 1x, 10x and 100x, each p50 ratio gated. Run by
/// `crates/api/benches/ratio.rs`'s `served_routes_are_flat` over the stores
/// `served` built.
pub(crate) fn exports_are_flat(stores: &mut [Served]) -> bool {
    flat("export", stores, |_served| {
        vec![
            // The oldest row is last in the newest-first order: the needle
            // is only in a body read to its end.
            ok_with(
                "1,000 rows, newest first",
                format!("/bars/export.csv?{QUERY}"),
                "09:15:00",
            ),
            ok_with(
                "1,000 rows, by close",
                format!("/bars/export.csv?{QUERY}&sort=c"),
                "09:15:00",
            ),
        ]
    })
}
