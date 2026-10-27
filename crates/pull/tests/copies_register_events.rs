//! THE COPIES REGISTER'S WALK IS AUDITED IN THE LOG (CC-5, D-7048).
//!
//! `pull::copies_register::note_walked` puts one `error` per hazard (an
//! audited event), one `warn` per copy root not walked whole and one `info`
//! summary, under the target `pull.copies`, so `/logs` shows every hazard and
//! refusal by name. The route and the verb call it after every walk.
//!
//! Its own test binary because `telemetry::install` is a process singleton:
//! no sibling test can write into this log and change the count. Every folder
//! and byte is invented.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};

use pull::copies_register::{Config, Host, LiveKind, LiveRoot, TARGET, note_walked, notes, walk};

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("brutex-copies-events-{}", std::process::id()));
    let _best_effort = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch root");
    dir
}

fn put(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("its folder");
    std::fs::write(path, bytes).expect("an invented file");
}

#[test]
fn every_hazard_refusal_and_walk_is_one_line_in_the_log() {
    let base = scratch();
    let logs = base.join("Logs");
    std::fs::create_dir_all(&logs).expect("the log folder");
    let config = telemetry::Config::new(&logs).with_min_level(telemetry::Level::Trace);
    let sink = telemetry::install(&config).expect("the only install in this binary");

    let archive = base.join("Options");
    let year = archive.join("2024.zip");
    put(&year, b"invented year zip, 2024");
    let queue = base.join("options-queue");
    std::fs::create_dir_all(&queue).expect("the queue");
    let part = queue.join("2024.preserved.1.zip.part");
    std::fs::hard_link(&year, &part).expect("a second name");
    let gone = base.join("went-away");
    let register = walk(
        &Host,
        &Config {
            copy_roots: vec![queue, gone.clone()],
            live: vec![LiveRoot {
                kind: LiveKind::NfoArchives,
                path: archive,
                named_by: "an invented archive root".to_owned(),
            }],
            unset_live: Vec::new(),
        },
        1_700_000_000_000_000,
    );
    assert_eq!(register.hazards.len(), 1);
    let expected = notes(&register);
    assert_eq!(expected.len(), 3, "a hazard, a refused root, the summary");

    note_walked(&register);
    sink.sync().expect("the log is durable before it is read");
    let log = std::fs::read_to_string(telemetry::current_path(&logs)).expect("the log");
    let target = format!("\"{TARGET}\"");
    let lines: Vec<&str> = log.lines().filter(|line| line.contains(&target)).collect();
    assert_eq!(lines.len(), expected.len(), "one line per note: {lines:#?}");
    let hazard = lines
        .iter()
        .find(|line| line.contains("live archive file hard-linked under another name"))
        .expect("the hazard's line");
    assert!(
        hazard.contains("\"error\""),
        "a hazard is an error: {hazard}"
    );
    assert!(
        hazard.contains("2024.preserved.1.zip.part") && hazard.contains("\"nlink\":\"2\""),
        "{hazard}"
    );
    let refused = lines
        .iter()
        .find(|line| line.contains("copy root not walked whole"))
        .expect("the refused root's line");
    assert!(refused.contains("\"warn\""), "{refused}");
    assert!(
        refused.contains(gone.to_str().expect("an invented path")),
        "{refused}"
    );
    let summary = lines
        .iter()
        .find(|line| line.contains("copies register walked"))
        .expect("the summary line");
    assert!(
        summary.contains("\"hazards\":\"1\"") && summary.contains("\"roots\":\"2\""),
        "{summary}"
    );
    assert_eq!(
        sink.health().dropped,
        0,
        "nothing dropped, so the count is exact"
    );

    let _best_effort = std::fs::remove_dir_all(&base);
}
