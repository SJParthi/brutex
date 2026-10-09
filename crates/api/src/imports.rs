//! `/imports.json` -- the GDFL import journal, tailed and paged by day: kind,
//! day, outcome, stage, reason, files, seconds, failures and the days the
//! census refused.
//!
//! # Registered, and not built
//!
//! The WP-01 seam registers the route ahead of WP-33a, the package that builds
//! it, so WP-33a edits this module and never the route table in `server.rs`.
//! Until it lands, every request is answered `501 Not Implemented` with the
//! reason `not built: WP-33a` -- loudly, never an empty body, and never a `404`
//! or a `[]` a reader could take for "there is nothing there". See
//! [`super::not_built`].

use super::{NotBuilt, not_built};

/// The package that builds this route.
const PACKAGE: &str = "WP-33a";

/// `GET /imports.json`: registered by the WP-01 seam and not built yet.
pub(super) async fn imports_json() -> NotBuilt {
    not_built("/imports.json", PACKAGE)
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

    /// **THE ROUTE IS REGISTERED AND ANSWERS `501`, NAMING WP-33a**, over the
    /// real router: GET carries the named reason in a JSON body, and HEAD answers
    /// the same status. Replaced by WP-33a's own tests when it builds the route.
    #[tokio::test]
    async fn the_route_is_registered_and_answers_not_built_by_name() {
        let dir = agreeing("imports-not-built");
        let built = Loaded::new(site("imports-not-built", &dir));
        assert_not_built(
            "imports-not-built",
            built,
            &[(
                "/imports.json",
                "/imports.json?feed=gdfl&from=2019-11-01&to=2019-11-30",
            )],
            "WP-33a",
        )
        .await;
    }
}
