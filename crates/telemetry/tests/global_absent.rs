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
    assert_eq!(telemetry::reserve_run_id(), None);
    assert_eq!(
        telemetry::emit_for_run(1, &Event::info("t", "no correlation sink")),
        Emitted::NotInstalled
    );
    assert!(telemetry::global().is_none());
    assert!(!Emitted::NotInstalled.is_written());
}

/// **`admits` WITH NOWHERE TO GO IS `false`, AND `emit_if!` EVALUATES NOTHING.**
///
/// [`telemetry::admits`] is the free function [`telemetry::emit_if`] gates on —
/// the whole mechanism that keeps a filtered event O(1) rather than
/// O(call-site fields). It had **no test at all**: `Sink::admits` was covered
/// and the free function that every macro expansion actually calls was not,
/// which is the "built and unreachable from a test" shape this crate has
/// already recorded twice.
///
/// The no-sink answer must be `false`. `true` would mean the macro evaluates
/// every argument in a binary that never configured logging — the exact cost it
/// exists to avoid, paid by the process least able to use the result.
///
/// The bound itself is proved by
/// `telemetry::sink::a_filtered_event_never_evaluates_its_arguments`, which
/// counts side effects rather than timing them, and measured flat by
/// `telemetry::bench::a_filtered_event_touches_nothing_and_stays_flat` (C-T-02).

#[test]
fn admits_with_no_sink_is_false_and_the_macro_evaluates_nothing() {
    assert!(
        telemetry::global().is_none(),
        "this binary installs nothing, so nothing can be installed"
    );

    for level in telemetry::LEVELS {
        assert!(
            !telemetry::admits(level, "pull.member"),
            "{level}: an event with nowhere to go is not admitted"
        );
    }
    assert!(
        !telemetry::admits(Level::Error, ""),
        "including an empty target"
    );

    // AND THE MACRO HONOURS IT. The counter can only move if the argument ran.
    let evaluated = std::sync::atomic::AtomicU64::new(0);
    let count = || {
        evaluated.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        7_u64
    };
    let outcome = telemetry::emit_if!(
        Level::Error,
        "pull.member",
        "did not land",
        "bars" => telemetry::Value::Uint(count()),
    );
    assert_eq!(
        outcome,
        Emitted::Filtered,
        "no sink means the gate refuses before the event is built"
    );
    assert_eq!(
        evaluated.load(std::sync::atomic::Ordering::Relaxed),
        0,
        "and the argument never ran — which is the whole point of the macro"
    );
}
