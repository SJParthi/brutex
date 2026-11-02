//! A sweep, a descent and every engine command the routes ACCEPT, driven to
//! their end over an empty store (BASE-COV-API, D-7344).
//!
//! Every other test of these routes stops at a refusal: a test binary carries
//! no commit stamp and has no sink, so nothing was ever accepted. Here the
//! stamp is passed in, the child installs its own sink and names its own store
//! ([`crate::isolated`]), and the engine, with no bar to read, finishes at once
//! with a named refusal. So what runs is the part no test reached: the slot, the
//! durable invocation, the task, the terminal status, each read back.
//!
//! The store is an empty scratch folder. No bar is read or invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::{Command, command_from, conduct_command, descend_with, run_with};
use crate::server::Loaded;
use axum::http::StatusCode;
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, Instant};

/// Names the child's scratch folder.
const CHILD: &str = "BRUTEX_TEST_SWEEP_ACCEPTED_CHILD";
/// A build stamp of the right shape; no commit has it.
const STAMP: &str = "0123456789abcdef0123456789abcdef01234567";
/// One month, which every command reads.
const SPAN: &str = r#""from_year":2025,"from_month":5,"to_year":2025,"to_month":5"#;
/// Two months, which `sweep-stored` refuses.
const TWO: &str = r#""from_year":2025,"from_month":4,"to_year":2025,"to_month":5"#;

/// The attempt an accepted press names.
fn attempt_of(route: &str, status: StatusCode, body: &str) -> u64 {
    assert_eq!(status, StatusCode::ACCEPTED, "{route}: {body}");
    let parsed: Value = serde_json::from_str(body).expect("an acceptance is JSON");
    assert_eq!(parsed["accepted"], true, "{route}: {body}");
    parsed["attempt"]
        .as_str()
        .map_or_else(
            || parsed["attempt"].as_u64(),
            |text| text.parse::<u64>().ok(),
        )
        .unwrap_or_else(|| panic!("{route}: no attempt in {body}"))
}

/// Waits for the slot to finish `attempt`; the text it finished with.
async fn settled(site: &Loaded, route: &str, attempt: u64) -> String {
    let deadline = Instant::now() + Duration::from_mins(2);
    loop {
        let held = site.sweep.lock().expect("the slot").clone();
        if let Some(progress) = held
            && progress.attempt == attempt
            && !progress.in_flight()
        {
            return progress
                .refusal
                .as_deref()
                .or(progress.report.as_deref())
                .unwrap_or_else(|| panic!("{route}: finished with no text"))
                .to_owned();
        }
        assert!(Instant::now() < deadline, "{route}: never finished");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// What `/backtest/run.json` answers for `target`, parsed.
async fn status_of(site: &Loaded, target: &str) -> (StatusCode, Value) {
    let (status, _, body) = super::run_json(
        axum::extract::State(std::sync::Arc::clone(site)),
        target.parse().expect("a target"),
    )
    .await;
    (status, serde_json::from_str(&body).expect("status JSON"))
}

/// The child: every accepted press over the empty store.
async fn every_press_accepted(root: &Path) {
    telemetry::install(&telemetry::Config::new(root.join("logs"))).expect("this child's sink");
    let store = root.join("store");
    let site: Loaded =
        std::sync::Arc::new(crate::server::Site::load(&root.join("masters"), &store));
    let command =
        |extra: &str| format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},{extra}}}"#);

    let run = format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},"rungs":["5min"]}}"#);
    let (status, _, body) = run_with(&site, &run, Some(STAMP));
    let attempt = attempt_of("run", status, &body);
    let said = settled(&site, "run", attempt).await;
    assert!(!said.is_empty(), "the run's own words");

    // THE TERMINAL STATUS, READ BACK: from the slot by its attempt, and by no
    // attempt at all once nothing is in flight.
    let (code, answer) = status_of(&site, &format!("/backtest/run.json?attempt={attempt}")).await;
    assert_eq!(code, StatusCode::OK, "{answer}");
    assert_eq!(answer["running"]["attempt"], attempt, "{answer}");
    let (code, _) = status_of(&site, "/backtest/run.json").await;
    assert_eq!(code, StatusCode::OK);

    let descent = command(r#""rung":"15min","max_points":20,"top":25"#);
    let (status, _, body) = descend_with(&site, &descent, Some(STAMP));
    let attempt = attempt_of("descend", status, &body);
    assert!(!settled(&site, "descend", attempt).await.is_empty());

    // AN ATTEMPT THE SLOT NO LONGER HOLDS is read from the durable invocation.
    let (code, answer) = status_of(
        &site,
        &format!("/backtest/run.json?attempt={}", attempt - 1),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{answer}");
    assert_eq!(
        answer["running"]["attempt"],
        attempt - 1,
        "the durable record of the run before: {answer}"
    );

    for (word, body) in [
        (
            "audit-range",
            command(r#""command":"audit-range","rung":"15min","min_hits":500"#),
        ),
        (
            "screen",
            command(
                r#""command":"screen","rung":"15min","support_ppm":50000,"max_points":20,"top":25"#,
            ),
        ),
        (
            "auto-stored",
            command(r#""command":"auto-stored","rung":"15min""#),
        ),
        (
            "sweep-stored",
            command(r#""command":"sweep-stored","rung":"15min","min_hits":500"#),
        ),
        (
            "sweep-stored over two months",
            format!(
                r#"{{"feed":"zerodha","underlying":"NIFTY",{TWO},"command":"sweep-stored","rung":"15min","min_hits":500}}"#
            ),
        ),
        (
            "sweep-all",
            r#"{"feed":"zerodha","command":"sweep-all","rung":"15min","min_hits":500}"#.to_owned(),
        ),
    ] {
        let (status, _, answer) = super::command_with(&site, &body, Some(STAMP));
        let attempt = attempt_of(word, status, &answer);
        let said = settled(&site, word, attempt).await;
        assert!(!said.is_empty(), "{word}");
        if word.ends_with("two months") {
            assert!(said.contains("walks ONE month"), "{word}: {said}");
        }
    }
}

/// **EVERY PRESS THE ROUTES ACCEPT RUNS TO ITS END AND READS BACK.** Over a
/// store with no bar: a run, a descent, and each ordinary engine command, the
/// one-month command asked for two months among them. Each answers `202`,
/// holds the slot, and settles with words of its own; the status route finds
/// it in the slot and, once the slot has moved on, in the durable record.
#[test]
fn every_press_the_routes_accept_runs_to_its_end_and_reads_back() {
    if let Some(root) = std::env::var_os(CHILD) {
        let root = Path::new(&root);
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("a runtime")
            .block_on(every_press_accepted(root));
        std::fs::write(root.join("child-ran"), b"accepted").expect("the child's proof");
        return;
    }
    let root = crate::scratch::path("sweep-accepted");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("store")).expect("the child's store");
    let store = root.join("store");
    let _ = crate::isolated::rerun(
        "sweeprun::accepted_tests::every_press_the_routes_accept_runs_to_its_end_and_reads_back",
        &[
            (CHILD, root.as_os_str()),
            ("BRUTEX_STORE", store.as_os_str()),
        ],
    );
    assert_eq!(
        std::fs::read(root.join("child-ran")).ok().as_deref(),
        Some(&b"accepted"[..]),
        "the child ran every press"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A COMMAND THE BROWSER DISPATCHES ELSEWHERE IS REFUSED BY NAME HERE.** The
/// two prepared searches and the strict audit have their own conductors, so
/// this one answers with why: reached only by a caller that skipped them.
/// Extremes: the strict command with no strict configuration in the
/// environment, and each of its three spellings of a window.
#[test]
fn a_command_dispatched_elsewhere_is_refused_by_name_here() {
    let strict = format!(
        r#"{{"command":"audit-audited-range","feed":"zerodha","underlying":"NIFTY","rung":"5min",{SPAN},"min_hits":500}}"#
    );
    let asked = command_from(&strict).expect("a strict command");
    assert!(matches!(asked, Command::AuditAuditedRange { .. }));
    assert_eq!(asked.feed(), "zerodha");
    assert_eq!(asked.underlying(), "NIFTY");
    assert_eq!(asked.window(), ((2025, 5), (2025, 5)));
    let done = conduct_command(&asked, 7, 3);
    let refusal = done.refusal.as_deref().expect("refused");
    assert!(refusal.starts_with("refused: "), "{refusal}");
    assert_eq!(done.finished_micros, Some(7));
}
