#![cfg(test)]
#![expect(
    clippy::expect_used,
    reason = "private admission fixtures must fail loudly"
)]
//! Admission must never be inferred by rewriting an old command's outcome.
#![allow(clippy::indexing_slicing, reason = "assert exact JSON fixture fields")]
use super::*;

fn fixture(name: &str) -> (std::path::PathBuf, telemetry::Sink) {
    let root = crate::scratch::path(name);
    std::fs::create_dir_all(&root).expect("private status fixture");
    let sink = telemetry::Sink::open(&telemetry::Config::new(root.join("logs")))
        .expect("private telemetry");
    (root, sink)
}

fn observed(root: &std::path::Path, now: i64) -> serde_json::Value {
    serde_json::from_str(&observed_status_with_admission(
        root,
        None,
        Some(&root.join("logs")),
        now,
    ))
    .expect("valid status JSON")
}

#[test]
fn ordinary_dispatch_cannot_claim_one_store_and_compute_into_another() {
    let (root, sink) = fixture("sweep-admission-store-identity");
    let other = root.join("other");
    std::fs::create_dir(&other).expect("private other store");
    let alias = root.join("alias");
    std::os::unix::fs::symlink(&other, &alias).expect("private store alias");
    assert!(require_same_execution_store(&root, &root).is_ok());
    assert!(require_same_execution_store(&alias, &other).is_ok());
    let mismatch =
        require_same_execution_store(&root, &other).expect_err("different stores refuse");
    assert!(mismatch.why().contains("no lease was claimed"));
    assert!(require_same_execution_store(&root, &root.join("missing")).is_err());
    assert!(!root.join(".sweep-execution-v1.lock").exists());
    drop(sink);
    std::fs::remove_dir_all(root).expect("private fixture cleanup");
}

#[test]
fn historical_inspection_does_not_own_the_execution_lease_or_become_completed() {
    let (root, sink) = fixture("sweep-admission-history");
    let _ = sink.emit_for_run(
        42,
        &telemetry::Event::info("cli.lifecycle", "command finished")
            .with("command", "help")
            .with("phase", "completed"),
    );
    let initial = observed(&root, 0);
    assert_eq!(initial["running"]["status"], "unknown");
    assert!(initial["running"]["report"].is_null());
    assert_eq!(initial["admission"]["available"], true);
    assert_eq!(initial["admission"]["scope"], "cooperating-store-writers");
    let lease = cli::execution_lease::Lease::acquire(&root).expect("private competing owner");
    let busy = observed(&root, 0);
    assert_eq!(busy["admission"]["available"], false);
    assert_eq!(busy["running"], initial["running"]);
    drop(lease);
    assert_eq!(observed(&root, 0)["admission"]["available"], true);
    drop(sink);
    std::fs::remove_dir_all(root).expect("private fixture cleanup");
}

#[test]
fn active_and_stale_unfinished_external_sweeps_block_even_when_the_lease_is_free() {
    let (root, sink) = fixture("sweep-admission-legacy-active");
    let _ = sink.emit_for_run(
        43,
        &telemetry::Event::info("cli.lifecycle", "command started")
            .with("command", "range-all")
            .with("phase", "running"),
    );
    assert_eq!(observed(&root, 0)["admission"]["available"], false);
    let stale = observed(&root, i64::MAX);
    assert_eq!(stale["running"]["status"], "unknown");
    assert_eq!(stale["admission"]["available"], false);
    let _ = sink.emit_for_run(
        43,
        &telemetry::Event::info("cli.lifecycle", "command finished")
            .with("command", "range-all")
            .with("phase", "refused"),
    );
    let ended = observed(&root, 0);
    assert_eq!(ended["running"]["status"], "refused");
    assert_eq!(ended["admission"]["available"], true);
    drop(sink);
    std::fs::remove_dir_all(root).expect("private fixture cleanup");
}

#[test]
fn corrupt_telemetry_and_unavailable_store_admission_never_enable_run() {
    use std::io::Write as _;
    let (root, sink) = fixture("sweep-admission-corruption");
    let _ = sink.emit_for_run(
        44,
        &telemetry::Event::info("cli.lifecycle", "command finished")
            .with("command", "help")
            .with("phase", "completed"),
    );
    let mut log = std::fs::OpenOptions::new()
        .append(true)
        .open(telemetry::current_path(&root.join("logs")))
        .expect("private log");
    log.write_all(b"damaged record\n")
        .expect("damaged telemetry fixture");
    log.sync_all().expect("fixture flush");
    let damaged = observed(&root, 0);
    assert_eq!(damaged["running"]["status"], "unknown");
    assert_eq!(damaged["admission"]["available"], false);
    let absent: serde_json::Value = serde_json::from_str(&observed_status_with_admission(
        &root.join("missing-store"),
        None,
        None,
        0,
    ))
    .expect("refusal JSON");
    assert_eq!(absent["admission"]["available"], false);
    drop(log);
    drop(sink);
    std::fs::remove_dir_all(root).expect("private fixture cleanup");
}

/// **A KILLED CLI SWEEP DOES NOT REFUSE EVERY LATER LAUNCH.** D-2764, sweep-1.
///
/// The CLI's durable invocation is begun and then abandoned with no terminal,
/// which is what Ctrl-C, a kill or an OOM leaves: `command started` is the
/// newest marker and nothing ends it. Before D-2764 that refused every browser
/// launch (and reported admission unavailable) even with the store's lease
/// free, which is the proof no cooperating sweep is alive. The boundaries stay
/// refused: a marker whose invocation this store's audit does not hold, and a
/// marker naming a different command than the invocation recorded.
#[test]
fn a_cli_sweep_that_died_without_its_terminal_does_not_block_a_lease_holder() {
    let (root, sink) = fixture("sweep-admission-abandoned");
    let attempt =
        cli::operation_audit::begin(&root, cli::operation_audit::Origin::Cli, "range-all")
            .expect("private durable invocation");
    let id = attempt.id();
    // THE PROCESS DIES HERE: no `Drop`, so no terminal record is written.
    std::mem::forget(attempt);
    let started = |run: u64, command: &str| {
        let _ = sink.emit_for_run(
            run,
            &telemetry::Event::info("cli.lifecycle", "command started")
                .with("command", command)
                .with("phase", "running"),
        );
    };

    // NOT IN THIS STORE'S AUDIT: refused, lease or no lease.
    started(id + 1, "range-all");
    let lease = cli::execution_lease::Lease::acquire(&root).expect("free lease");
    let unknown = observe_elsewhere(&root.join("logs"), i64::MAX);
    assert!(!unknown.launch_clear);
    assert!(admit_external(&root, &unknown).is_err());
    drop(lease);
    assert_eq!(observed(&root, i64::MAX)["admission"]["available"], false);

    // A DIFFERENT COMMAND WORD than the invocation recorded: refused.
    started(id, "sweep-stored");
    let lease = cli::execution_lease::Lease::acquire(&root).expect("free lease");
    let other = observe_elsewhere(&root.join("logs"), i64::MAX);
    assert!(admit_external(&root, &other).is_err());
    drop(lease);

    // THE DEAD INVOCATION ITSELF, fresh and stale alike: admitted while the
    // lease is held, and the status still reports what the log shows.
    started(id, "range-all");
    for now in [0, i64::MAX] {
        let lease = cli::execution_lease::Lease::acquire(&root).expect("free lease");
        let dead = observe_elsewhere(&root.join("logs"), now);
        assert!(!dead.launch_clear, "the log itself is not rewritten");
        admit_external(&root, &dead).expect("the free lease proves it is not running");
        drop(lease);
        let status = observed(&root, now);
        assert_eq!(status["admission"]["available"], true, "{status}");
        assert_ne!(status["running"]["status"], "completed", "{status}");
        assert!(
            status["admission"]["why"]
                .as_str()
                .is_some_and(|why| why.contains(&id.to_string())),
            "the admission names the invocation it found ended: {status}"
        );
    }
    assert!(
        cli::operation_audit::read(&root, id)
            .expect("audit")
            .is_some_and(|record| !record.phase.terminal()),
        "no terminal was invented for the dead invocation"
    );
    drop(sink);
    std::fs::remove_dir_all(root).expect("private fixture cleanup");
}
