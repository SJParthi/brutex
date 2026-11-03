//! BASE-COV-PULL: stop lines of any other shape, what a stop asks of a held
//! lock whose claim is not there yet, and the small doors of a stop flag
//! (Route B, recipe v4). Every run id is invented.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::gdfl_fixtures::scratch;

#[test]
fn a_stop_line_of_any_other_shape_is_refused_by_name() {
    let good = StopRequest::parse("stop run=7 requested=8 by=cli\n").unwrap();
    assert_eq!((good.run, good.requested), (7, 8));
    assert_eq!(StopRequest::parse(&good.line()).unwrap(), good);
    assert!(
        StopRequest::parse("stop run=7")
            .unwrap_err()
            .contains("not a stop line")
    );
    for bad in [
        "stop run= requested=8 by=cli",
        "stop run=x requested=8 by=cli",
        "stop run=123456789012345678901 requested=8 by=cli",
        "stop run=99999999999999999999 requested=8 by=cli",
        "stop run=7 requested= by=cli",
        "stop runs=7 requested=8 by=cli",
    ] {
        assert!(
            StopRequest::parse(bad)
                .unwrap_err()
                .contains("is not a number"),
            "{bad}"
        );
    }
    assert!(
        StopRequest::parse("stop run=7 requested=8 by=nobody")
            .unwrap_err()
            .contains("by= is not one of")
    );
}

#[test]
fn a_stop_file_past_its_bound_or_not_text_is_no_stop() {
    let root = scratch("cov-stop-file");
    let path = root.join("a.stop");
    std::fs::write(&path, vec![b'x'; PROBE_READ + 1]).unwrap();
    assert_eq!(read_stop(&path), None);
    let bytes = root.join("b.stop");
    std::fs::write(&bytes, [0xFF, 0xFE]).unwrap();
    assert_eq!(read_stop(&bytes), None);
    assert_eq!(read_stop(&root.join("none")), None);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_stop_asks_who_is_allowed_to_ask() {
    let root = scratch("cov-stop-by");
    let refused = request_stop(&root, StopTarget::Run(7), "nobody", 1).unwrap_err();
    assert!(matches!(refused, StopRefusal::ByUnknown { .. }));
    let none = request_stop(&root, StopTarget::Kind("stocks"), "cli", 1).unwrap_err();
    assert!(matches!(none, StopRefusal::NotLive { .. }));
    std::fs::remove_dir_all(root).unwrap();
}

// ───── BASE-COV-PULL repair 2 (D-7334) ─────

/// A stop names the run it belongs to and the file it reads.
#[test]
fn a_stop_names_its_run_and_its_file() {
    let control = StopControl::new(PathBuf::from("/invented/imports/a.stop"), 41);
    assert_eq!(control.run(), 41);
    assert_eq!(control.path(), Path::new("/invented/imports/a.stop"));
}

/// A kind lock held while its claim is still being written: a stop of that
/// kind is refused by the lock's name (the run cannot be named yet), a stop
/// by run id passes it over, and a held lock whose claim does not parse is
/// passed over too, so both end not live (extremes at once: an empty claim
/// and a garbled one, on one v4 store).
#[test]
fn a_stop_of_a_kind_whose_claim_is_being_written_is_refused_by_its_lock() {
    let store = scratch("cov-stop-writing");
    std::fs::create_dir_all(store.join("manifest/v4/gdfl")).unwrap();
    let writing = locks::RunLocks::take(&store, "indices", 0).unwrap();
    let garbled = locks::RunLocks::take(&store, "stocks", 0).unwrap();
    std::fs::write(store.join(locks::kind_lock("stocks")), b"not a claim").unwrap();
    let refused = request_stop(&store, StopTarget::Kind("indices"), "cli", 1).unwrap_err();
    let StopRefusal::ClaimBeingWritten { lock } = &refused else {
        panic!("{refused:?}");
    };
    assert_eq!(*lock, locks::kind_lock("indices"));
    let by_run = request_stop(&store, StopTarget::Run(7), "cli", 1).unwrap_err();
    assert!(matches!(by_run, StopRefusal::NotLive { .. }), "{by_run:?}");
    let of_stocks = request_stop(&store, StopTarget::Kind("stocks"), "cli", 1).unwrap_err();
    assert!(
        matches!(of_stocks, StopRefusal::NotLive { .. }),
        "{of_stocks:?}"
    );
    writing.release().unwrap();
    garbled.release().unwrap();
}

// ───── BASE-COV-PULL-PS1 (D-7335) ─────

/// An invented claim of `kind` for `run`, its batch naming that run (a claim
/// with an empty batch is unreadable, `locks::value_ok`).
fn claim(kind: &str, run: u64) -> locks::Claim {
    locks::Claim {
        run,
        mode: "import".to_owned(),
        kind: kind.to_owned(),
        batch: format!("{kind}:{run}"),
        source: "tickstore".to_owned(),
        from: "2024-04-01".to_owned(),
        to: "2024-04-01".to_owned(),
        filter: "*".to_owned(),
        verify: "off".to_owned(),
        restate: false,
        repair: "0".to_owned(),
        workers: 1,
        auto_resume: false,
        pid: 4242,
        host: "-".to_owned(),
        exe: "-".to_owned(),
        by: "api".to_owned(),
        started: 0,
        resumes: 0,
        resume_day: "-".to_owned(),
        legacy_ack: false,
        unstamped: false,
        request: "-".to_owned(),
    }
}

/// A v4 store holding a census claim of run 41 under the stocks lock and an
/// index claim of run 43, both held.
fn two_claims(tag: &str) -> (PathBuf, locks::RunLocks, locks::RunLocks) {
    let store = scratch(tag);
    std::fs::create_dir_all(store.join("manifest/v4/gdfl")).unwrap();
    let census = locks::RunLocks::take(&store, "stocks", 0).unwrap();
    census.write_claim(&claim("census", 41)).unwrap();
    let held = locks::RunLocks::take(&store, "indices", 0).unwrap();
    held.write_claim(&claim("indices", 43)).unwrap();
    (store, census, held)
}

/// **A STOP PASSES OVER EVERY HELD CLAIM IT DOES NOT NAME**: a census claim
/// (never stoppable) under a stop of another run or of its lock's kind, and
/// an index claim under a stop of another run or another kind, are each
/// passed over, so each stop is not live; the stop of run 43 is answered with
/// the claim it found and its request written. Extremes at once: a census
/// claim under a kind lock a stop names, and two held claims, neither named.
#[test]
fn a_stop_passes_over_every_held_claim_it_does_not_name() {
    let (store, census, held) = two_claims("ps1-stop-others");
    for target in [
        StopTarget::Run(42),
        StopTarget::Run(44),
        StopTarget::Kind("stocks"),
        StopTarget::Kind("options"),
    ] {
        let refused = request_stop(&store, target, "cli", 1).unwrap_err();
        assert!(
            matches!(refused, StopRefusal::NotLive { .. }),
            "{refused:?}"
        );
    }
    let answer = request_stop(&store, StopTarget::Run(43), "cli", 9).unwrap();
    assert_eq!(
        (answer.run, answer.kind.as_str(), answer.request.requested),
        (43, "indices", 9)
    );
    census.release().unwrap();
    held.release().unwrap();
    std::fs::remove_dir_all(store).unwrap();
}

/// **A SECOND ASK KEEPS THE FIRST REQUEST**: the stop says who asked first
/// and when, whoever asks again.
#[test]
fn a_second_ask_keeps_the_first_request() {
    let control = StopControl::new(PathBuf::from("/invented/imports/a.stop"), 41);
    control.ask("operator", 5);
    control.ask("cli", 9);
    assert!(control.requested());
    let kept = control.request().unwrap();
    assert_eq!(
        (kept.run, kept.requested, kept.by.as_str()),
        (41, 5, "operator")
    );
}

/// **A STOP ASKED WITH A SINK INSTALLED IS LOGGED UNDER THE STOPPED RUN'S KEY
/// AND GIVES THE KEY BACK**: in a process of its own (the one install there),
/// the stop of run 43 claims that run's key for its line and releases it, so
/// the key is free again after. Extremes at once: a sink at its lowest level,
/// and a store holding a claim the stop passes over too.
#[test]
fn a_stop_asked_with_a_sink_installed_gives_the_runs_key_back() {
    crate::support::where_permission_binds(
        "gdfl_run::stop::cov_tests::a_stop_asked_with_a_sink_installed_gives_the_runs_key_back",
        a_stop_asked_with_a_sink_installed_gives_the_runs_key_back_body,
    );
}

fn a_stop_asked_with_a_sink_installed_gives_the_runs_key_back_body() {
    let home = scratch("ps1-stop-sink-home");
    let sink =
        telemetry::install(&telemetry::Config::new(&home).with_min_level(telemetry::Level::Trace))
            .unwrap();
    let (store, census, held) = two_claims("ps1-stop-sink");
    let answer = request_stop(&store, StopTarget::Run(43), "cli", 9).unwrap();
    assert_eq!(answer.run, 43);
    assert_eq!(sink.run(), 0, "the stop gave the run's key back");
    census.release().unwrap();
    held.release().unwrap();
    std::fs::remove_dir_all(store).unwrap();
}
