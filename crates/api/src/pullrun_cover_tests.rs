//! Rows of `pullrun.rs` no test reached before BASE-COV-API repair 3 (shard
//! AS1, D-8661): a run's durable id once one is taken, the calendar of
//! record's horizon seen from a day before its last sourced year, and the
//! telemetry sink's run key released when the run that holds it ends.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a whole-file test module; fixtures are bounded and a failed assertion is the test failing"
)]

use super::*;

/// The run document `status_json` writes for `progress` seen from `today`.
fn document(progress: &Progress, today: i64) -> serde_json::Value {
    serde_json::from_str(&status_json(progress, Some(today), None)).expect("valid JSON")
}

/// **A RUN'S DOCUMENT NAMES ITS DURABLE ID ONCE ONE IS TAKEN, AND A HORIZON
/// FAR FROM ITS END IS NOT ENDING SOON** (BASE-COV-API repair 3, D-8661):
/// `run_id_json` and `status_json`'s `endsSoon` false side. Extremes: no id
/// taken (`null`); the first id; the largest; the day before the record's
/// last sourced year starts (not ending soon); that year's first day (ending
/// soon); and, at once, a run holding an id seen from the epoch's first day
/// (the id and a far horizon together); out of the box, a day as early as an
/// `i64` holds (its days left saturate, never wrap).
#[test]
fn a_runs_document_names_its_id_and_a_far_horizon_is_not_ending_soon() {
    assert_eq!(run_id_json(0), "null");
    assert_eq!(run_id_json(1), "1");
    assert_eq!(run_id_json(u64::MAX), "18446744073709551615");
    let starts = pull::calendar::LAST_SOURCED_YEAR_STARTS;
    let idle = Progress::default();
    let before = document(&idle, starts - 1);
    assert_eq!(before["calendar"]["endsSoon"], false, "{before}");
    assert_eq!(before["calendar"]["ended"], false, "{before}");
    assert_eq!(
        before["calendar"]["daysLeft"],
        pull::calendar::SOURCED_LAST_DAY - (starts - 1),
        "{before}"
    );
    assert_eq!(before["runId"], serde_json::Value::Null, "{before}");
    let first = document(&idle, starts);
    assert_eq!(first["calendar"]["endsSoon"], true, "{first}");
    let held = Progress {
        run_id: 42,
        ..Progress::default()
    };
    let epoch = document(&held, 0);
    assert_eq!(epoch["runId"], 42, "{epoch}");
    assert_eq!(epoch["calendar"]["endsSoon"], false, "{epoch}");
    let earliest = document(&idle, i64::MIN);
    assert_eq!(earliest["calendar"]["daysLeft"], i64::MAX, "{earliest}");
    assert_eq!(earliest["calendar"]["endsSoon"], false, "{earliest}");
}

/// The child's marker: set, the test is the child that installs the sink.
const SINK_CHILD: &str = "BRUTEX_TEST_PULLRUN_SINK_CHILD";

/// **A RUN'S TELEMETRY KEY IS RELEASED WHEN THE RUN THAT HOLDS IT ENDS, AND A
/// RUN THAT LOST THE CLAIM RELEASES NOTHING** (BASE-COV-API repair 3,
/// D-8661): `SinkRun`'s claim and drop. The sink is a process global, so the
/// claims run in a child of this binary that installs its own (as
/// `sweeprun_accepted_tests.rs` does). Extremes: id 0 (claims nothing); the
/// first id held, then dropped (released); a second run while one holds the
/// key (claims nothing, and its drop leaves the holder's key); the largest
/// id; and, at once, another caller releasing an id it never held while the
/// largest is held (the key stays), then the holder dropped (released); out
/// of the box, a run claimed again after its own release (claimed afresh).
#[test]
fn a_runs_sink_key_is_released_when_it_ends_and_only_its_own() {
    if let Some(root) = std::env::var_os(SINK_CHILD) {
        let root = std::path::Path::new(&root);
        let sink = telemetry::install(&telemetry::Config::new(root.join("logs")))
            .expect("this child's sink");
        let none = SinkRun::claim(0);
        assert_eq!((none.0, sink.run()), (None, 0), "id 0 claims nothing");
        drop(none);
        let held = SinkRun::claim(1);
        assert_eq!((held.0, sink.run()), (Some(1), 1));
        let lost = SinkRun::claim(2);
        assert_eq!(lost.0, None, "the key is held");
        drop(lost);
        assert_eq!(sink.run(), 1, "a run that lost the claim releases nothing");
        drop(held);
        assert_eq!(sink.run(), 0, "released when the holder ends");
        let largest = SinkRun::claim(u64::MAX);
        assert_eq!(sink.run(), u64::MAX);
        sink.release_run(3);
        assert_eq!(sink.run(), u64::MAX, "another id's release leaves it");
        drop(largest);
        assert_eq!(sink.run(), 0);
        let again = SinkRun::claim(1);
        assert_eq!((again.0, sink.run()), (Some(1), 1), "claimed afresh");
        drop(again);
        assert_eq!(sink.run(), 0);
        std::fs::write(root.join("child-ran"), b"sink").expect("the child's proof");
        return;
    }
    let root = crate::scratch::path("pullrun-cover-sink");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    let _ = crate::isolated::rerun(
        "pullrun::cover_tests::a_runs_sink_key_is_released_when_it_ends_and_only_its_own",
        &[(SINK_CHILD, root.as_os_str())],
    );
    assert_eq!(
        std::fs::read(root.join("child-ran")).ok().as_deref(),
        Some(&b"sink"[..]),
        "the child ran every claim"
    );
    let _ = std::fs::remove_dir_all(&root);
}
