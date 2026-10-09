//! `/aliases.json` -- the alias and symbol-lineage table: renamed stocks and
//! index aliases, each with its source or marked UNVERIFIED.
//!
//! # Registered, and not built
//!
//! The WP-01 seam registers the route ahead of WP-20, the package that builds
//! it, so WP-20 edits this module and never the route table in `server.rs`.
//! Until it lands, every request is answered `501 Not Implemented` with the
//! reason `not built: WP-20` -- loudly, never an empty body, and never a `404`
//! or a `[]` a reader could take for "there is nothing there". See
//! [`super::not_built`].

use super::{NotBuilt, not_built};

/// The package that builds this route.
const PACKAGE: &str = "WP-20";

/// `GET /aliases.json`: registered by the WP-01 seam and not built yet.
pub(super) async fn aliases_json() -> NotBuilt {
    not_built("/aliases.json", PACKAGE)
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

    /// **THE ROUTE IS REGISTERED AND ANSWERS `501`, NAMING WP-20**, over the
    /// real router: GET carries the named reason in a JSON body, and HEAD answers
    /// the same status. Replaced by WP-20's own tests when it builds the route.
    #[tokio::test]
    async fn the_route_is_registered_and_answers_not_built_by_name() {
        let dir = agreeing("aliases-not-built");
        let built = Loaded::new(site("aliases-not-built", &dir));
        assert_not_built(
            "aliases-not-built",
            built,
            &[("/aliases.json", "/aliases.json?symbol=ZOMATO")],
            "WP-20",
        )
        .await;
    }
}
