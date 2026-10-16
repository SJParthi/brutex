#![cfg(test)]
#![expect(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes: a test \
              that cannot panic cannot fail"
)]
#![allow(clippy::indexing_slicing, reason = "assert exact JSON fixture fields")]
//! THE CLI LOG IS READ ONLY WHEN IT CHANGED (p99-7, AF-06).
//!
//! `/backtest/run.json` walked the CLI log up to 4 MiB on every 2 s poll. The
//! walk is now remembered under the stamp of every file in the log's set and
//! judged again at each poll's own `now`; these tests drive
//! `CliLogMemory::evidence` with a private memory per test, so no test sees
//! another's walk, and count the bytes each poll read.
use super::*;
use std::time::{Duration, Instant};

/// A fresh, empty fixture directory. Never deletes: a name already taken (a
/// dead process with the same id) moves on to the next suffix.
fn fresh(name: &str) -> std::path::PathBuf {
    for n in 0_u32.. {
        let dir = crate::scratch::path(&format!("{name}-{n}"));
        if !dir.exists() {
            std::fs::create_dir_all(&dir).expect("private CLI log fixture");
            return dir;
        }
    }
    unreachable!("u32 suffixes are not exhausted by one test")
}

fn sink(dir: &std::path::Path) -> telemetry::Sink {
    telemetry::Sink::open(&telemetry::Config::new(dir)).expect("private telemetry")
}

fn marker(sink: &telemetry::Sink, run: u64, message: &str, phase: &str) {
    let event = telemetry::Event::info("cli.lifecycle", message)
        .with("phase", phase)
        .with("command", "sweep-stored");
    assert_eq!(sink.emit_for_run(run, &event), telemetry::Emitted::Written);
}

fn activity(sink: &telemetry::Sink, run: u64, message: &str) {
    let event = telemetry::Event::info("cli.sweep", message);
    assert_eq!(sink.emit_for_run(run, &event), telemetry::Emitted::Written);
}

/// A running sweep: its `command started` and one activity record.
fn running_log(name: &str) -> (std::path::PathBuf, telemetry::Sink) {
    let dir = fresh(name);
    let sink = sink(&dir);
    marker(&sink, 61, "command started", "running");
    activity(&sink, 61, "rung 1min loaded");
    (dir, sink)
}

fn poll(
    memory: &CliLogMemory,
    dir: &std::path::Path,
    at: Instant,
    now: i64,
) -> (ExternalObservation, LogAnswer) {
    let (evidence, answer) = memory.evidence(dir, at, now);
    (judge(&evidence, now), answer)
}

#[test]
fn an_unchanged_cli_log_is_answered_without_a_read() {
    let (dir, _sink) = running_log("af06-unchanged");
    let memory = CliLogMemory::new();
    let start = Instant::now();
    let now = telemetry::now_millis();
    let (first, walked) = poll(&memory, &dir, start, now);
    assert_eq!(walked.answered, Answered::Walk);
    assert!(
        walked.bytes_read > 0,
        "premise: the first poll reads the log"
    );
    assert!(
        first.body.contains(r#""status":"running""#),
        "{}",
        first.body
    );

    let (second, remembered) = poll(&memory, &dir, start + Duration::from_secs(2), now + 2_000);
    assert_eq!(remembered.answered, Answered::Memory);
    assert_eq!(remembered.bytes_read, 0, "the second call reads 0 bytes");
    assert_eq!(remembered.walks, 1);
    assert_eq!(remembered.from_memory, 1);
    assert_eq!(remembered.walked_at_unix_millis, now);
    // THE SAME EVIDENCE, JUDGED AT ITS OWN TIME: only the age moved.
    assert_eq!(
        second.body,
        first.body.replace(
            &format!(r#""age_millis":{}"#, now - first.at_millis.expect("at")),
            &format!(
                r#""age_millis":{}"#,
                now + 2_000 - first.at_millis.expect("at")
            ),
        )
    );
    // And the judgement is not remembered: past the stale bound the same
    // remembered evidence says `unknown`, not `running`.
    let (stale, late) = poll(
        &memory,
        &dir,
        start + Duration::from_secs(3),
        now + STALE_AFTER_MILLIS + 1_000,
    );
    assert_eq!(late.answered, Answered::Memory);
    assert!(
        stale.body.contains(r#""status":"unknown""#),
        "{}",
        stale.body
    );
    // What is remembered is exactly what a fresh walk reads, and judging it is
    // exactly what the uncached `observe_elsewhere` says.
    let fresh_walk = read_evidence(&dir);
    assert!(fresh_walk.bytes_read > 0);
    assert_eq!(fresh_walk.evidence, memory_evidence(&memory));
    assert_eq!(observe_elsewhere(&dir, now).body, first.body);
}

fn memory_evidence(memory: &CliLogMemory) -> Evidence {
    memory
        .kept
        .lock()
        .expect("unpoisoned")
        .as_ref()
        .map(|entry| entry.evidence.as_ref().clone())
        .expect("a remembered walk")
}

#[test]
fn a_grown_log_is_walked_again() {
    let (dir, sink) = running_log("af06-grown");
    let memory = CliLogMemory::new();
    let at = Instant::now();
    let now = telemetry::now_millis();
    let (_, first) = poll(&memory, &dir, at, now);
    assert_eq!(first.answered, Answered::Walk);
    activity(&sink, 61, "rung 2min loaded");
    let (grown, answer) = poll(&memory, &dir, at, now);
    assert_eq!(
        answer.answered,
        Answered::Walk,
        "a grown log is walked again"
    );
    assert!(answer.bytes_read > first.bytes_read);
    assert!(grown.body.contains("rung 2min loaded"), "{}", grown.body);
    marker(&sink, 61, "command finished", "completed");
    let (done, answer) = poll(&memory, &dir, at, now);
    assert_eq!(answer.answered, Answered::Walk);
    assert!(
        done.body.contains(r#""status":"completed""#),
        "{}",
        done.body
    );
    assert!(done.launch_clear);
    let (_, again) = poll(&memory, &dir, at, now);
    assert_eq!((again.answered, again.bytes_read), (Answered::Memory, 0));
}

#[test]
fn a_rotated_log_is_walked_again() {
    let dir = fresh("af06-rotated");
    let small = telemetry::Sink::open(&telemetry::Config {
        max_file_bytes: telemetry::MIN_FILE_BYTES,
        ..telemetry::Config::new(&dir)
    })
    .expect("small-file telemetry");
    marker(&small, 62, "command started", "running");
    let memory = CliLogMemory::new();
    let at = Instant::now();
    let now = telemetry::now_millis();
    let (before, first) = poll(&memory, &dir, at, now);
    assert_eq!(first.answered, Answered::Walk);
    assert_eq!(small.health().rotations, 0, "premise: nothing rolled yet");
    // ROLLED BY THE SINK ITSELF: records of another run until the file rolls.
    for n in 0..1_000 {
        if small.health().rotations > 0 {
            break;
        }
        activity(&small, 99, &format!("unrelated traffic {n}"));
    }
    assert!(
        small.health().rotations > 0,
        "premise: the sink rolled the file"
    );
    let (after, answer) = poll(&memory, &dir, at, now);
    assert_eq!(
        answer.answered,
        Answered::Walk,
        "a rolled log is walked again"
    );
    assert!(answer.bytes_read > 0);
    assert!(after.body.contains(r#""attempt":62"#), "{}", after.body);
    assert_eq!(before.in_flight, after.in_flight);
    let (_, still) = poll(&memory, &dir, at, now);
    assert_eq!(still.answered, Answered::Memory);
}

/// A log holding one running sweep's `command started`, and the same bytes
/// with the phase word `running` swapped for `unknown`: both seven bytes, so
/// the two logs have exactly the same length and say different things.
fn same_length_pair(name: &str) -> (std::path::PathBuf, Vec<u8>) {
    let dir = fresh(name);
    let writer = sink(&dir);
    marker(&writer, 63, "command started", "running");
    drop(writer);
    let running = std::fs::read(telemetry::current_path(&dir)).expect("fixture log");
    let text = String::from_utf8(running.clone()).expect("utf-8 log");
    assert_eq!(
        text.matches("running").count(),
        1,
        "premise: one phase word"
    );
    let unknown = text.replacen("running", "unknown", 1).into_bytes();
    assert_eq!(running.len(), unknown.len(), "premise: the same length");
    assert_ne!(running, unknown, "premise: different bytes");
    (dir, unknown)
}

#[test]
fn a_log_replaced_by_one_of_the_same_length_is_walked_again() {
    let (dir, unknown) = same_length_pair("af06-replaced");
    let memory = CliLogMemory::new();
    let at = Instant::now();
    let now = telemetry::now_millis();
    let (first, walked) = poll(&memory, &dir, at, now);
    assert_eq!(walked.answered, Answered::Walk);
    assert!(
        first.body.contains(r#""status":"running""#),
        "{}",
        first.body
    );
    // REPLACED BY RENAME, the way an editor or a restore replaces a file: a
    // new inode at the same length.
    let staged = dir.join("staged.tmp");
    std::fs::write(&staged, &unknown).expect("staged replacement");
    std::fs::rename(&staged, telemetry::current_path(&dir)).expect("replace the log");
    let (second, answer) = poll(&memory, &dir, at, now);
    assert_eq!(
        answer.answered,
        Answered::Walk,
        "a replaced log is walked again"
    );
    assert!(
        second.body.contains(r#""status":"unknown""#),
        "{}",
        second.body
    );
}

#[test]
fn a_log_rewritten_in_place_at_the_same_length_is_walked_again_even_with_its_old_mtime() {
    let (dir, unknown) = same_length_pair("af06-inplace");
    let memory = CliLogMemory::new();
    let at = Instant::now();
    let now = telemetry::now_millis();
    let path = telemetry::current_path(&dir);
    let old = std::fs::metadata(&path)
        .expect("log")
        .modified()
        .expect("mtime");
    let (_, walked) = poll(&memory, &dir, at, now);
    assert_eq!(walked.answered, Answered::Walk);
    // Past one tick of a coarse filesystem clock, so the change time moves.
    std::thread::sleep(Duration::from_millis(20));
    std::fs::write(&path, &unknown).expect("rewrite in place");
    // THE CLOCK STEPPED BACK: the modification time is put back to what the
    // remembered walk saw. Inode and length are unchanged too; the change
    // time is what still differs.
    std::fs::File::options()
        .write(true)
        .open(&path)
        .expect("log")
        .set_modified(old)
        .expect("restore mtime");
    let (second, answer) = poll(&memory, &dir, at, now);
    assert_eq!(answer.answered, Answered::Walk, "{answer:?}");
    assert!(
        second.body.contains(r#""status":"unknown""#),
        "{}",
        second.body
    );
}

#[test]
fn a_truncated_log_is_walked_again() {
    let (dir, sink) = running_log("af06-truncated");
    let memory = CliLogMemory::new();
    let at = Instant::now();
    let now = telemetry::now_millis();
    let (_, first) = poll(&memory, &dir, at, now);
    assert_eq!(first.answered, Answered::Walk);
    drop(sink);
    let path = telemetry::current_path(&dir);
    let len = std::fs::metadata(&path).expect("log").len();
    std::fs::File::options()
        .write(true)
        .open(&path)
        .expect("log")
        .set_len(len / 2)
        .expect("truncate");
    let (cut, answer) = poll(&memory, &dir, at, now);
    assert_eq!(
        answer.answered,
        Answered::Walk,
        "a truncated log is walked again"
    );
    assert!(
        cut.body.contains(r#""status":"unknown""#),
        "a torn log is damage, never a remembered running sweep: {}",
        cut.body
    );
}

#[test]
fn a_zero_byte_log_and_a_missing_directory_are_remembered_until_they_change() {
    // ZERO BYTES.
    let dir = fresh("af06-empty");
    std::fs::write(telemetry::current_path(&dir), b"").expect("empty log");
    let memory = CliLogMemory::new();
    let at = Instant::now();
    let (empty, first) = poll(&memory, &dir, at, 0);
    assert_eq!((first.answered, first.bytes_read), (Answered::Walk, 0));
    assert_eq!(
        empty.body, NO_SWEEP,
        "an empty log names no sweep and claims nothing more"
    );
    let (_, second) = poll(&memory, &dir, at, 0);
    assert_eq!(second.answered, Answered::Memory);
    let writer = sink(&dir);
    marker(&writer, 64, "command started", "running");
    let (grown, third) = poll(&memory, &dir, at, telemetry::now_millis());
    assert_eq!(third.answered, Answered::Walk);
    assert!(grown.body.contains(r#""attempt":64"#), "{}", grown.body);

    // NO DIRECTORY AT ALL: every file absent is a stamp too.
    let parent = fresh("af06-missing");
    let missing = parent.join("logs");
    let memory = CliLogMemory::new();
    let (none, first) = poll(&memory, &missing, at, 0);
    assert_eq!(first.answered, Answered::Walk);
    assert_eq!(none.body, NO_SWEEP);
    let (_, second) = poll(&memory, &missing, at, 0);
    assert_eq!((second.answered, second.bytes_read), (Answered::Memory, 0));
    std::fs::create_dir_all(&missing).expect("the directory appears");
    let late = sink(&missing);
    marker(&late, 65, "command started", "running");
    let (appeared, third) = poll(&memory, &missing, at, telemetry::now_millis());
    assert_eq!(third.answered, Answered::Walk);
    assert!(
        appeared.body.contains(r#""attempt":65"#),
        "{}",
        appeared.body
    );
}

#[test]
fn a_log_that_cannot_be_described_is_never_remembered() {
    // A FILE WHERE THE DIRECTORY SHOULD BE: every `stat` under it fails with
    // something other than "absent".
    let parent = fresh("af06-not-a-dir");
    let dir = parent.join("logs");
    std::fs::write(&dir, b"not a directory").expect("a file in the way");
    assert!(
        log_stamp(&dir).is_none(),
        "premise: the set cannot be described"
    );
    let memory = CliLogMemory::new();
    let at = Instant::now();
    for _ in 0..2 {
        let (observed, answer) = poll(&memory, &dir, at, 0);
        assert_eq!(answer.answered, Answered::WalkNotKept);
        assert!(observed.uncertain, "{}", observed.body);
        assert!(
            observed.body.contains("unreadable files"),
            "{}",
            observed.body
        );
    }
    assert!(memory.kept.lock().expect("unpoisoned").is_none());
}

#[test]
fn a_log_that_changes_during_the_walk_answers_that_poll_and_is_not_remembered() {
    let (dir, sink) = running_log("af06-mid-walk");
    let memory = CliLogMemory::new();
    let at = Instant::now();
    let now = telemetry::now_millis();
    let (evidence, answer) = memory.evidence_by(&dir, at, now, |dir| {
        let walk = read_evidence(dir);
        // A CLI writes while the walk is reading.
        activity(&sink, 61, "rung 3min loaded");
        walk
    });
    assert_eq!(answer.answered, Answered::WalkNotKept);
    assert!(answer.bytes_read > 0);
    assert!(judge(&evidence, now).body.contains("rung 1min loaded"));
    let (next, again) = poll(&memory, &dir, at, now);
    assert_eq!(again.answered, Answered::Walk, "the next poll walks again");
    assert!(next.body.contains("rung 3min loaded"), "{}", next.body);
}

#[test]
fn the_monotonic_bound_rewalks_and_a_wall_clock_step_back_changes_nothing() {
    let (dir, _sink) = running_log("af06-clock");
    let memory = CliLogMemory::new();
    let at = Instant::now();
    let now = telemetry::now_millis();
    let (_, first) = poll(&memory, &dir, at, now);
    assert_eq!(first.answered, Answered::Walk);
    // THE WALL CLOCK STEPPED BACK AN HOUR: memory is keyed on files and the
    // monotonic clock, so it still answers, and the age is clamped at zero.
    let (back, answer) = poll(&memory, &dir, at, now - 3_600_000);
    assert_eq!(answer.answered, Answered::Memory);
    assert!(back.body.contains(r#""age_millis":0"#), "{}", back.body);
    // A monotonic instant earlier than the walk (as a test can construct) is
    // not an expiry either.
    let earlier = at.checked_sub(Duration::from_secs(1)).unwrap_or(at);
    assert_eq!(
        poll(&memory, &dir, earlier, now).1.answered,
        Answered::Memory
    );
    // Just inside and just past the bound.
    let inside = at + REWALK_AFTER.saturating_sub(Duration::from_millis(1));
    assert_eq!(
        poll(&memory, &dir, inside, now).1.answered,
        Answered::Memory
    );
    let (_, past) = poll(&memory, &dir, at + REWALK_AFTER, now);
    assert_eq!(
        past.answered,
        Answered::Walk,
        "the bound re-walks an unchanged log"
    );
    assert!(past.bytes_read > 0);
    assert_eq!(past.walks, 2);
}

#[test]
fn one_memory_follows_the_directory_it_was_asked_about() {
    let (one, _a) = running_log("af06-dir-one");
    let (two, _b) = running_log("af06-dir-two");
    let memory = CliLogMemory::new();
    let at = Instant::now();
    let now = telemetry::now_millis();
    assert_eq!(poll(&memory, &one, at, now).1.answered, Answered::Walk);
    assert_eq!(poll(&memory, &two, at, now).1.answered, Answered::Walk);
    assert_eq!(poll(&memory, &two, at, now).1.answered, Answered::Memory);
    assert_eq!(poll(&memory, &one, at, now).1.answered, Answered::Walk);
}

#[test]
fn concurrent_polls_of_an_unchanged_log_agree() {
    let (dir, _sink) = running_log("af06-concurrent");
    let memory = CliLogMemory::new();
    let at = Instant::now();
    let now = telemetry::now_millis();
    let bodies: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| poll(&memory, &dir, at, now).0.body))
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("poll thread"))
            .collect()
    });
    assert!(
        bodies.windows(2).all(|pair| pair[0] == pair[1]),
        "{bodies:?}"
    );
    let walks = memory.walks.load(std::sync::atomic::Ordering::Relaxed);
    let hits = memory
        .from_memory
        .load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(walks + hits, 8);
    assert!(walks >= 1);
}

#[test]
fn concurrent_polls_after_a_change_make_one_walk() {
    // AF-06 r1 follow-up: eight polls that miss at once used to make eight
    // 4 MiB walks. The walk here is slow on purpose, so every poll has missed
    // before the first walk is done; single flight leaves exactly one walk and
    // seven answers from its memory.
    let (dir, _sink) = running_log("af06-single-flight");
    let memory = CliLogMemory::new();
    let at = Instant::now();
    let now = telemetry::now_millis();
    let walked = std::sync::atomic::AtomicU64::new(0);
    let start = std::sync::Barrier::new(8);
    let answers: Vec<(Answered, String)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    start.wait();
                    let (evidence, answer) = memory.evidence_by(&dir, at, now, |dir| {
                        walked.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        std::thread::sleep(Duration::from_millis(150));
                        read_evidence(dir)
                    });
                    (answer.answered, judge(&evidence, now).body)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("poll thread"))
            .collect()
    });
    assert_eq!(
        walked.load(std::sync::atomic::Ordering::Relaxed),
        1,
        "{answers:?}"
    );
    assert_eq!(
        answers
            .iter()
            .filter(|(answered, _)| *answered == Answered::Walk)
            .count(),
        1
    );
    assert_eq!(
        answers
            .iter()
            .filter(|(answered, _)| *answered == Answered::Memory)
            .count(),
        7
    );
    assert!(answers.windows(2).all(|pair| pair[0].1 == pair[1].1));
    assert!(answers[0].1.contains("rung 1min loaded"), "{answers:?}");
    // A poll that hits memory never waits on the walk gate: hold it and ask.
    let gate = memory
        .walking
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(poll(&memory, &dir, at, now).1.answered, Answered::Memory);
    drop(gate);
}

#[test]
fn the_route_body_says_where_its_cli_evidence_came_from() {
    let root = fresh("af06-route");
    let dir = root.join("logs");
    std::fs::create_dir_all(&dir).expect("logs");
    let writer = sink(&dir);
    marker(&writer, 66, "command finished", "completed");
    let memory = CliLogMemory::new();
    let at = Instant::now();
    let now = telemetry::now_millis();
    let body = |at| -> serde_json::Value {
        serde_json::from_str(&observed_status_remembered(
            &memory,
            &root,
            None,
            Some(&dir),
            now,
            at,
        ))
        .expect("one JSON object")
    };
    let first = body(at);
    assert_eq!(first["cli_log"]["answered_from"], "walk", "{first}");
    assert!(first["cli_log"]["bytes_read"].as_u64().expect("bytes") > 0);
    assert_eq!(first["admission"]["schema_version"], 1, "{first}");
    assert_eq!(first["running"]["status"], "completed", "{first}");
    let second = body(at);
    assert_eq!(second["cli_log"]["answered_from"], "memory", "{second}");
    assert_eq!(second["cli_log"]["bytes_read"], 0, "{second}");
    assert_eq!(second["cli_log"]["walks"], 1, "{second}");
    assert_eq!(second["cli_log"]["answered_from_memory"], 1, "{second}");
    assert_eq!(
        second["cli_log"]["rewalk_after_millis"], 300_000,
        "{second}"
    );
    assert_eq!(second["running"], first["running"]);
    assert_eq!(second["admission"], first["admission"]);
    // The uncached door every older caller uses answers the same.
    let plain: serde_json::Value = serde_json::from_str(&observed_status_with_admission(
        &root,
        None,
        Some(&dir),
        now,
    ))
    .expect("one JSON object");
    assert_eq!(plain["running"], first["running"]);
    assert!(plain.get("cli_log").is_none());
    // NOT CONFIGURED is said, not implied.
    let none: serde_json::Value = serde_json::from_str(&observed_status_remembered(
        &memory, &root, None, None, now, at,
    ))
    .expect("one JSON object");
    assert_eq!(none["cli_log"]["answered_from"], "not-configured", "{none}");
    assert_eq!(none["running"]["status"], "unknown", "{none}");
    // And the production route reaches the process memory, not a fresh walk.
    let source = include_str!("sweeprun.rs");
    let route = source
        .split_once("pub async fn run_json")
        .map_or("", |(_, rest)| rest);
    let route = route
        .split_once("\nfn requested_attempt")
        .map_or("", |(head, _)| head);
    assert!(
        route.contains("observed_status_remembered(\n            &CLI_LOG_MEMORY,"),
        "{route}"
    );
    assert!(
        !route.contains("observed_status_with_admission("),
        "{route}"
    );
}
