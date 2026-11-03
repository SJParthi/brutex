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
    // THE ATTEMPT IS WRITTEN AS A STRING, always (`sweeprun::accepted`): a
    // number in its place is a change of the answer, and fails here.
    let text = parsed["attempt"]
        .as_str()
        .expect("an acceptance writes its attempt as a string");
    text.parse::<u64>()
        .expect("the attempt string is a whole number")
}

/// Waits for the slot to finish `attempt`; the text it finished with.
///
/// From the press's acceptance on, the slot holds that attempt and no other:
/// these presses run one at a time, each waited for here before the next.
async fn settled(site: &Loaded, route: &str, attempt: u64) -> String {
    let deadline = Instant::now() + Duration::from_mins(2);
    loop {
        let progress = site
            .sweep
            .lock()
            .expect("the slot")
            .clone()
            .expect("an accepted press holds the slot");
        assert_eq!(
            progress.attempt, attempt,
            "{route}: the slot holds this press"
        );
        if !progress.in_flight() {
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

    // AN ATTEMPT AT EACH EDGE OF ITS RANGE: the largest u64 and the smallest
    // positive one, neither ever started, are each an unknown attempt said in
    // words, never another attempt's status and never a completion.
    for asked in [u64::MAX, 1] {
        let (code, answer) = status_of(&site, &format!("/backtest/run.json?attempt={asked}")).await;
        assert_eq!(code, StatusCode::OK, "{asked}: {answer}");
        assert_eq!(answer["running"]["status"], "unknown", "{asked}: {answer}");
        assert_eq!(
            answer["running"]["requested_attempt"],
            asked.to_string(),
            "{asked}: {answer}"
        );
        assert_eq!(answer["running"]["in_flight"], false, "{asked}: {answer}");
    }
    // AN ATTEMPT THAT IS NO CANONICAL POSITIVE u64 IS A 400 BEFORE ANY READ:
    // zero, a leading zero, one past the largest, empty, another field, and
    // (out of the box) the field given twice.
    for query in [
        "attempt=0",
        "attempt=01",
        "attempt=18446744073709551616",
        "attempt=184467440737095516150",
        "attempt=",
        "when=1",
        "attempt=1&attempt=1",
    ] {
        let (code, answer) = status_of(&site, &format!("/backtest/run.json?{query}")).await;
        assert_eq!(code, StatusCode::BAD_REQUEST, "{query}: {answer}");
        assert_eq!(answer["running"]["status"], "unknown", "{query}: {answer}");
    }

    // A PRESS WHILE AN ATTEMPT IS IN FLIGHT IS REFUSED, on each of the three
    // doors at once, and leaves the slot as it was; the status with no
    // attempt then answers the one in flight. The slot is set by hand: over a
    // store with no bar no press stays in flight long enough to press again.
    let finished = site
        .sweep
        .lock()
        .expect("the slot")
        .clone()
        .expect("the last press");
    let mut flying = finished.clone();
    flying.finished_micros = None;
    *site.sweep.lock().expect("the slot") = Some(flying.clone());
    for (word, (status, _, answer)) in [
        ("run", run_with(&site, &run, Some(STAMP))),
        ("descend", descend_with(&site, &descent, Some(STAMP))),
        (
            "audit-range",
            super::command_with(
                &site,
                &command(r#""command":"audit-range","rung":"15min","min_hits":500"#),
                Some(STAMP),
            ),
        ),
    ] {
        assert_eq!(status, StatusCode::CONFLICT, "{word}: {answer}");
        assert!(answer.contains(r#""accepted":false"#), "{word}: {answer}");
        assert!(answer.contains("already running"), "{word}: {answer}");
    }
    let held = site
        .sweep
        .lock()
        .expect("the slot")
        .clone()
        .expect("still held");
    assert_eq!(
        held.attempt, flying.attempt,
        "a refused press takes no attempt"
    );
    assert!(
        held.in_flight(),
        "a refused press does not finish the one in flight"
    );
    let (code, answer) = status_of(&site, "/backtest/run.json").await;
    assert_eq!(code, StatusCode::OK, "{answer}");
    assert_eq!(answer["running"]["in_flight"], true, "{answer}");
    assert_eq!(
        answer["running"]["attempt"], flying.attempt,
        "the one in flight, from memory: {answer}"
    );
    *site.sweep.lock().expect("the slot") = Some(finished);
}

/// **EVERY PRESS THE ROUTES ACCEPT RUNS TO ITS END AND READS BACK.** Over a
/// store with no bar: a run, a descent, and each ordinary engine command, the
/// one-month command asked for two months among them. Each answers `202`,
/// holds the slot, and settles with words of its own; the status route finds
/// it in the slot and, once the slot has moved on, in the durable record.
/// Extremes (D-7346): the largest and the smallest attempt, neither started;
/// zero, a leading zero, one past the largest, an empty attempt and another
/// field, each a `400`; and, at once, all three doors pressed while an attempt
/// is in flight, each a `409` that leaves the slot as it was; out of the box,
/// the attempt field given twice.
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

/// Names the crowding children's proof folder.
const CROWD_CHILD: &str = "BRUTEX_TEST_SWEEP_CROWD_CHILD";

/// Runs `body` alone in a child of this test binary (`crate::isolated`): a
/// crowd of the detail pool in this process would refuse every other test
/// that reads through it while it stood. The child proves it ran.
fn in_a_child<F, Fut>(name: &str, body: F)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    if let Some(proof) = std::env::var_os(CROWD_CHILD) {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("a runtime")
            .block_on(body());
        std::fs::write(proof, b"crowded").expect("the child's proof");
        return;
    }
    let proof = crate::scratch::path(&format!("sweep-crowd-{}", name.len()));
    let _ = std::fs::remove_file(&proof);
    let _ = crate::isolated::rerun(name, &[(CROWD_CHILD, proof.as_os_str())]);
    assert_eq!(
        std::fs::read(&proof).ok().as_deref(),
        Some(&b"crowded"[..]),
        "the child ran {name}"
    );
    let _ = std::fs::remove_file(&proof);
}

/// **EACH PRESS DOOR ANSWERS THROUGH THE DETAIL POOL, AND A FULL POOL IS
/// SAID** (BASE-COV-API, D-7346). The three handlers the router mounts, not
/// the `_with` halves every other test calls: in a test build, which carries
/// no commit stamp, each refuses in the stamp's own words before the slot;
/// with every slot of the detail pool taken, each says the admission is
/// unavailable and dispatches nothing. Neither leaves a press in the slot.
#[test]
fn each_press_door_answers_through_its_pool_and_says_when_it_is_full() {
    in_a_child(
        "sweeprun::accepted_tests::each_press_door_answers_through_its_pool_and_says_when_it_is_full",
        press_doors,
    );
}

/// The child half of the test above: alone in its process, so its crowd of
/// the detail pool refuses no other test.
async fn press_doors() {
    assert!(
        cli::commit_stamp().is_none(),
        "the premise: a test build carries no commit stamp"
    );
    let root = crate::scratch::path("sweep-doors");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    let site: Loaded = std::sync::Arc::new(crate::server::Site::load(&root.join("masters"), &root));
    let doors = || {
        let site = std::sync::Arc::clone(&site);
        async move {
            let state = || axum::extract::State(std::sync::Arc::clone(&site));
            let base = format!(r#""feed":"zerodha","underlying":"NIFTY",{SPAN}"#);
            vec![
                (
                    "run",
                    super::run(state(), format!(r#"{{{base},"rungs":["5min"]}}"#)).await,
                ),
                (
                    "descend",
                    super::descend(
                        state(),
                        format!(r#"{{{base},"rung":"15min","max_points":20,"top":25}}"#),
                    )
                    .await,
                ),
                (
                    "command",
                    super::command(
                        state(),
                        format!(
                            r#"{{{base},"command":"audit-range","rung":"15min","min_hits":500}}"#
                        ),
                    )
                    .await,
                ),
            ]
        }
    };
    for (door, (status, _, body)) in doors().await {
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{door}: {body}");
        assert!(body.contains(r#""accepted":false"#), "{door}: {body}");
        assert!(body.contains("BRUTEX_COMMIT"), "{door}: {body}");
    }
    let crowd = crate::detail::crowd(crate::detail::Named::Detail);
    let full = doors().await;
    drop(crowd);
    for (door, (status, _, body)) in full {
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{door}: {body}");
        assert!(body.contains("admission is unavailable"), "{door}: {body}");
        assert!(
            body.contains("no engine command was dispatched"),
            "{door}: {body}"
        );
    }
    assert!(
        site.sweep.lock().expect("the slot").is_none(),
        "no refused press took the slot"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **AN ATTEMPT THE DURABLE RECORD CANNOT BE ASKED FOR IS UNAVAILABLE, NEVER
/// A COMPLETION** (BASE-COV-API, D-7346). The status route reads an attempt
/// the slot does not hold from the invocation journal: over a store root that
/// is gone, the journal's own refusal is said with a `503`; with every slot
/// of the detail pool taken, the read is not queued and says so. Extremes: the
/// first durable attempt, and the largest.
#[test]
fn an_attempt_the_record_cannot_be_asked_for_is_unavailable() {
    in_a_child(
        "sweeprun::accepted_tests::an_attempt_the_record_cannot_be_asked_for_is_unavailable",
        attempt_reads,
    );
}

/// The child half of the test above, alone in its process for its crowd.
async fn attempt_reads() {
    let root = crate::scratch::path("sweep-status-gone");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    let site: Loaded = std::sync::Arc::new(crate::server::Site::load(
        &root.join("masters"),
        &root.join("no-store-here"),
    ));
    let first = cli::operation_audit::ID_BASE + 1;
    for attempt in [first, u64::MAX] {
        let target = format!("/backtest/run.json?attempt={attempt}");
        let (code, answer) = status_of(&site, &target).await;
        assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE, "{attempt}: {answer}");
        assert_eq!(answer["running"]["status"], "unknown", "{answer}");
        assert!(
            answer["running"]["why"]
                .as_str()
                .expect("the words")
                .contains("store root is absent"),
            "{answer}"
        );
        let crowd = crate::detail::crowd(crate::detail::Named::Detail);
        let (code, answer) = status_of(&site, &target).await;
        drop(crowd);
        assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE, "{attempt}: {answer}");
        assert!(
            answer["running"]["why"]
                .as_str()
                .expect("the words")
                .contains("persistent invocation read unavailable"),
            "{answer}"
        );
    }
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
