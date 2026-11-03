#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

//! BASE-COV-PULL-PS2 (D-7336): the filing's arms a whole import day does not
//! reach on its own, each called with the input that reaches it. Every
//! fixture is invented.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::gdfl_run::stop::StopControl;

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-filing-ps2-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos())
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// THE LOSS PROBE: the root the run began on answers nothing; another
/// directory at its path, a file there, and nothing there each say why.
#[test]
fn the_loss_probe_names_another_directory_a_file_and_nothing() {
    use std::os::unix::fs::MetadataExt as _;
    let root = scratch("LOSS");
    let meta = std::fs::metadata(&root).unwrap();
    let id = (meta.dev(), meta.ino());
    assert_eq!(store_root_lost(&root, Some(id)), None);
    assert_eq!(
        store_root_lost(&root, None),
        None,
        "a fresh store: a directory"
    );
    let other = store_root_lost(&root, Some((id.0, id.1 ^ 1))).expect("another directory");
    assert!(
        other.contains("is another directory than the run began on"),
        "{other}"
    );
    let file = root.join("a-file");
    std::fs::write(&file, b"invented").unwrap();
    let not_dir = store_root_lost(&file, Some(id)).expect("a file");
    assert!(
        not_dir.contains("is no longer a directory: the store's root went away"),
        "{not_dir}"
    );
    let gone = store_root_lost(&root.join("gone"), None).expect("nothing there");
    assert!(gone.contains("could not reach"), "{gone}");
    std::fs::remove_dir_all(&root).unwrap();
}

/// THE STOP OF A STORE THAT TAKES NO WRITES carries the host's refusal first
/// and then every fix it names.
#[test]
fn the_not_writable_stop_names_its_refusal_and_each_fix() {
    let said = not_writable_said("invented: Permission denied (os error 13)");
    assert!(
        said.starts_with(
            "invented: Permission denied (os error 13); the store does not take writes"
        ),
        "{said}"
    );
    for fix in [
        "chflags nouchg <file>",
        "chmod -N <file>",
        "remount the volume read-write",
    ] {
        assert!(said.contains(fix), "{fix}: {said}");
    }
}

/// A FILE NOT TAKEN AFTER AN EARLIER FILE'S FAULT names that fault: a full
/// disk, a lost store, and any other code a store that takes no writes.
#[test]
fn a_file_not_taken_after_an_earlier_fault_names_that_fault() {
    let day = Day::new(2019, 11, 14).unwrap();
    for (code, word) in [
        (1, "disk_full"),
        (2, "store_lost"),
        (3, "store_not_writable"),
        (9, "store_not_writable"),
    ] {
        let refused = not_taken(ImportKind::Options, day, code);
        assert!(
            refused.failure.why.contains(&format!(
                "an earlier file of the day met {word}, so the run ends"
            )),
            "{code}: {}",
            refused.failure.why
        );
    }
}

/// WORKERS ASKED TO STOP BEFORE THEIR FIRST FILE take no job, and the pool
/// answers the nothing they did.
#[test]
fn workers_asked_to_stop_take_no_job() {
    let root = scratch("STOP");
    let stop = StopControl::new(root.join("stop"), 7);
    stop.ask("cli", 0);
    let gate = BeginGate::new(None, 2);
    let ran = AtomicUsize::new(0);
    let jobs = [1, 2, 3];
    let done = on_workers(
        2,
        &jobs,
        &gate,
        &mut || Ok(1),
        &|job: &i32| {
            ran.fetch_add(1, Ordering::Relaxed);
            *job
        },
        Some(&stop),
    )
    .expect("no refusal");
    assert!(done.is_empty(), "{done:?}");
    assert_eq!(ran.load(Ordering::Relaxed), 0);
    std::fs::remove_dir_all(&root).unwrap();
}

/// A JOB THAT PANICS PANICS ITS CALLER with its own payload: a worker's
/// fault is never swallowed into a short list.
#[test]
fn a_job_that_panics_panics_its_caller() {
    let gate = BeginGate::new(None, 2);
    let jobs = [1, 2];
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        on_workers(
            2,
            &jobs,
            &gate,
            &mut || Ok(1),
            &|job: &i32| {
                assert!(*job != 2, "invented job fault");
                *job
            },
            None,
        )
    }));
    let payload = caught.expect_err("the panic comes back");
    let said = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .expect("a panic message");
    assert_eq!(said, "invented job fault");
}

/// A BEGIN ATTEMPT THAT LEFT NEITHER A LINE NOR A REFUSAL is said as the line
/// not written: a worker that asked never waits for ever and never files
/// under a line nobody wrote.
#[test]
fn a_begin_attempt_that_left_no_line_and_no_refusal_is_said_not_written() {
    let gate = std::sync::Arc::new(BeginGate::new(None, 1));
    let asker = {
        let gate = std::sync::Arc::clone(&gate);
        std::thread::spawn(move || gate.line())
    };
    loop {
        let state = gate.lock();
        if state.asked {
            break;
        }
        drop(state);
        std::thread::yield_now();
    }
    {
        let mut state = gate.lock();
        state.asked = false;
        state.attempts += 1;
        gate.changed.notify_all();
    }
    let refused = asker.join().unwrap().expect_err("no line");
    assert!(
        matches!(
            &refused,
            ImportRefusal::Journal { why } if why == "the day's begin line was not written"
        ),
        "{refused:?}"
    );
}
