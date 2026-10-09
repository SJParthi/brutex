//! `/sources.json`, `/sources/bars.json`, `/sources/day.json` -- the All
//! sources view of one instrument: the rung table, one combined timeline per
//! rung, and the day reconciliation, every row keeping its feed.
//!
//! # Registered, and not built
//!
//! The WP-01 seam registers these routes ahead of WP-37, the package that
//! builds them, so WP-37 edits this module and never the route table in
//! `server.rs`. Until it lands, every request is answered `501 Not Implemented`
//! with the reason `not built: WP-37` -- loudly, never an empty body, and never
//! a `404` or a `[]` a reader could take for "there is nothing there". See
//! [`super::not_built`].

use super::{NotBuilt, not_built};

/// The package that builds these routes.
const PACKAGE: &str = "WP-37";

/// `GET /sources.json`: registered by the WP-01 seam and not built yet.
pub(super) async fn sources_json() -> NotBuilt {
    not_built("/sources.json", PACKAGE)
}

/// `GET /sources/bars.json`: registered by the WP-01 seam and not built yet.
pub(super) async fn sources_bars_json() -> NotBuilt {
    not_built("/sources/bars.json", PACKAGE)
}

/// `GET /sources/day.json`: registered by the WP-01 seam and not built yet.
pub(super) async fn sources_day_json() -> NotBuilt {
    not_built("/sources/day.json", PACKAGE)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use crate::server::Loaded;
    use crate::server::tests::{agreeing, assert_not_built, site};

    /// **EACH ROUTE IS REGISTERED AND ANSWERS `501`, NAMING WP-37**, over the
    /// real router: GET carries the named reason in a JSON body, and HEAD answers
    /// the same status. Replaced by WP-37's own tests when it builds them.
    #[tokio::test]
    async fn each_route_is_registered_and_answers_not_built_by_name() {
        let dir = agreeing("sources-not-built");
        let built = Loaded::new(site("sources-not-built", &dir));
        assert_not_built(
            "sources-not-built",
            built,
            &[
                ("/sources.json", "/sources.json?exchange=NSE&segment=INDEX&symbol=NIFTY"),
                ("/sources/bars.json", "/sources/bars.json?exchange=NSE&segment=INDEX&symbol=NIFTY&rung=1min&offset=0&limit=50"),
                ("/sources/day.json", "/sources/day.json?exchange=NSE&segment=INDEX&symbol=NIFTY&from=2019-03-01&to=2019-03-31"),
            ],
            "WP-37",
        )
        .await;
    }
}
