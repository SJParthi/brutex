//! `/provenance.json` -- where one stored instrument-month came from: its
//! sidecar when one exists, or the record reconstructed from the import or pull
//! journal and labelled as reconstructed.
//!
//! # Registered, and not built
//!
//! The WP-01 seam registers the route ahead of WP-38b, the package that builds
//! it, so WP-38b edits this module and never the route table in `server.rs`.
//! Until it lands, every request is answered `501 Not Implemented` with the
//! reason `not built: WP-38b` -- loudly, never an empty body, and never a `404`
//! or a `[]` a reader could take for "there is nothing there". See
//! [`super::not_built`].

use super::{NotBuilt, not_built};

/// The package that builds this route.
const PACKAGE: &str = "WP-38b";

/// `GET /provenance.json`: registered by the WP-01 seam and not built yet.
pub(super) async fn provenance_json() -> NotBuilt {
    not_built("/provenance.json", PACKAGE)
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

    /// **THE ROUTE IS REGISTERED AND ANSWERS `501`, NAMING WP-38b**, over the
    /// real router: GET carries the named reason in a JSON body, and HEAD answers
    /// the same status. Replaced by WP-38b's own tests when it builds the route.
    #[tokio::test]
    async fn the_route_is_registered_and_answers_not_built_by_name() {
        let dir = agreeing("provenance-not-built");
        let built = Loaded::new(site("provenance-not-built", &dir));
        assert_not_built(
            "provenance-not-built",
            built,
            &[(
                "/provenance.json",
                "/provenance.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&timeframe=1s&month=2019-03",
            )],
            "WP-38b",
        )
        .await;
    }
}
