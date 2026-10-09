//! `/chain.json` -- one underlying's option chain for one expiry at one
//! instant, every strike with its spot, moneyness and greeks flags.
//!
//! # Registered, and not built
//!
//! The WP-01 seam registers the route ahead of WP-32, the package that builds
//! it, so WP-32 edits this module and never the route table in `server.rs`.
//! Until it lands, every request is answered `501 Not Implemented` with the
//! reason `not built: WP-32` -- loudly, never an empty body, and never a `404`
//! or a `[]` a reader could take for "there is nothing there". See
//! [`super::not_built`].

use super::{NotBuilt, not_built};

/// The package that builds this route.
const PACKAGE: &str = "WP-32";

/// `GET /chain.json`: registered by the WP-01 seam and not built yet.
pub(super) async fn chain_json() -> NotBuilt {
    not_built("/chain.json", PACKAGE)
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

    /// **THE ROUTE IS REGISTERED AND ANSWERS `501`, NAMING WP-32**, over the
    /// real router: GET carries the named reason in a JSON body, and HEAD answers
    /// the same status. Replaced by WP-32's own tests when it builds the route.
    #[tokio::test]
    async fn the_route_is_registered_and_answers_not_built_by_name() {
        let dir = agreeing("chain-not-built");
        let built = Loaded::new(site("chain-not-built", &dir));
        assert_not_built(
            "chain-not-built",
            built,
            &[(
                "/chain.json",
                "/chain.json?feed=gdfl&underlying=NIFTY&expiry=2019-12-26&at=2019-12-02T10:00:00",
            )],
            "WP-32",
        )
        .await;
    }
}
