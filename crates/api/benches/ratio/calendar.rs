//! Gate 8 for `/calendar.json`: the feed's exchange calendar and one series'
//! calendar cost the same whatever the months hold and however large the
//! census is (R3-14, RA-064).
//!
//! The answer is built once per feed generation and kept (WP-35, D-3336); the
//! build is the `cold` figure, and what is gated is the kept answer every
//! later request is.

use super::served::{Served, flat, ok_with};

/// The routes this module times, read by `.github/gates_routes.rs`.
pub(crate) const ROUTES: &[&str] = &["/calendar.json"];

/// Every target of calendar at 1x, 10x and 100x, each p50 ratio gated. Run by
/// `crates/api/benches/ratio.rs`'s `main` over the stores `served` built.
pub(crate) fn calendars_are_flat(stores: &mut [Served]) -> bool {
    flat("calendar", stores, |_served| {
        vec![
            ok_with(
                "the feed's calendar",
                "/calendar.json?feed=gdfl".to_owned(),
                "sessions",
            ),
            ok_with(
                "NIFTY's calendar",
                "/calendar.json?feed=gdfl&symbol=NIFTY".to_owned(),
                "sessions",
            ),
            ok_with(
                "the calendar of record",
                "/calendar.json?feed=gdfl&span=record".to_owned(),
                "sessions",
            ),
        ]
    })
}
