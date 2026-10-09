//! `/census/facets.json` -- the selection-scoped lists the /db pickers need --
//! underlyings, expiries, strikes, months, per-rung counts, first and last
//! spans -- through census map probes, with `?since=<generation>` deltas.
//!
//! # Registered, and not built
//!
//! The WP-01 seam registers the route ahead of WP-30, the package that builds
//! it, so WP-30 edits this module and never the route table in `server.rs`.
//! Until it lands, every request is answered `501 Not Implemented` with the
//! reason `not built: WP-30` -- loudly, never an empty body, and never a `404`
//! or a `[]` a reader could take for "there is nothing there". See
//! [`super::not_built`].

use super::{NotBuilt, not_built};

/// The package that builds this route.
const PACKAGE: &str = "WP-30";

/// `GET /census/facets.json`: registered by the WP-01 seam and not built yet.
pub(super) async fn facets_json() -> NotBuilt {
    not_built("/census/facets.json", PACKAGE)
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

    /// **THE ROUTE IS REGISTERED AND ANSWERS `501`, NAMING WP-30**, over the
    /// real router: GET carries the named reason in a JSON body, and HEAD answers
    /// the same status. Replaced by WP-30's own tests when it builds the route.
    #[tokio::test]
    async fn the_route_is_registered_and_answers_not_built_by_name() {
        let dir = agreeing("facets-not-built");
        let built = Loaded::new(site("facets-not-built", &dir));
        assert_not_built(
            "facets-not-built",
            built,
            &[(
                "/census/facets.json",
                "/census/facets.json?feed=gdfl&segment=FNO&underlying=NIFTY",
            )],
            "WP-30",
        )
        .await;
    }
}
