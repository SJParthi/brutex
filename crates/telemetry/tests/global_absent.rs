//! What happens before anything installs a sink.
//!
//! # Why this is its own test binary
//!
//! [`telemetry::install`] writes to a `OnceLock`, which is **per process**.
//! The unit tests in `src/lib.rs` install one, and `cargo test` runs every
//! test in a crate's unit-test binary in the same process with no ordering
//! guarantee — so a test asserting "nothing is installed" inside that binary
//! would pass or fail depending on which thread got there first. That is the
//! definition of a flaky test, and a flaky test is worse than no test because
//! it trains a person to re-run the suite.
//!
//! An integration test is its own binary and its own process. This one never
//! installs anything, so the state it asserts is the only state it can be in.

use telemetry::{Emitted, Event, Level};

/// AN EVENT WITH NOWHERE TO GO SAYS SO, AND DOES NOT PANIC.
///
/// The alternative implementations are both wrong: a panic would mean a
/// library that logs kills any binary that has not configured logging, and a
/// silent `Written` would mean a caller believes its events are on disk when
/// no file exists at all.
#[test]
fn emitting_with_no_sink_installed_is_reported_and_is_not_a_panic() {
    assert!(
        telemetry::global().is_none(),
        "this binary installs nothing, so nothing can be installed"
    );

    for event in [
        Event::trace("t", "quietest"),
        Event::error("t", "loudest"),
        Event::new(Level::Info, "pull.http", "with fields")
            .with("status", 200u32)
            .with("ok", true),
    ] {
        assert_eq!(
            telemetry::emit(&event),
            Emitted::NotInstalled,
            "an uninstalled sink is a state, not a failure and not a success"
        );
    }

    // And it is still none afterwards: `emit` does not install one by
    // accident, which would make the first event decide where every later one
    // goes.
    assert!(telemetry::global().is_none());
    assert!(!Emitted::NotInstalled.is_written());
}
