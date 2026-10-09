//! A closed stderr kills nothing, and its loss is recorded (r53-1, D-4413).
//!
//! The parent test re-runs this binary as a child whose stderr is the write end
//! of a pipe whose read end is already closed — the state the r53 audit found
//! `eprintln!` panicking on ("failed printing to stderr: Broken pipe"). The
//! child must pass: no panic, every line counted, the first loss written to
//! the log once a sink exists, and a sink's own notice named in its health.
//!
//! Its own binary because the child installs the process-wide sink, which is
//! once per process, and asserts the process-wide counter from zero.

#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use telemetry::{Config, Emitted, Event, Query, Sink, Target};

const CHILD: &str = "BRUTEX_CLOSED_STDERR_CHILD";

/// A destination that refuses every append, so the sink has a notice to give.
#[derive(Debug)]
struct Refuses;

impl Target for Refuses {
    fn append(&mut self, _bytes: &[u8]) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::StorageFull,
            "no space left on device",
        ))
    }

    fn sync(&self) -> std::io::Result<()> {
        Ok(())
    }

    fn reopen(&mut self, _path: &std::path::Path) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_closed_stderr_is_counted_and_logged_and_never_panics() {
    if std::env::var_os(CHILD).is_some() {
        child();
        return;
    }
    let (reader, writer) = std::io::pipe().expect("a pipe");
    drop(reader);
    let output = std::process::Command::new(std::env::current_exe().expect("this binary"))
        .args([
            "--exact",
            "a_closed_stderr_is_counted_and_logged_and_never_panics",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, "1")
        .stderr(writer)
        .output()
        .expect("the child runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed") && stdout.contains("CHILD-RAN"),
        "the child failed or ran nothing with a closed stderr: {:?}\n{stdout}",
        output.status
    );
}

fn child() {
    // Before any sink: the line is lost and counted, and with nowhere to
    // write the event, the next failure is left to name it.
    assert_eq!(telemetry::unprinted(), 0);
    assert!(!telemetry::stderr_line(format_args!("before any sink")));
    assert_eq!(telemetry::unprinted(), 1);

    let dir = std::env::temp_dir().join(format!(
        "brutex-telemetry-closed-stderr-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&dir);
    let sink = telemetry::install(&Config::new(&dir)).expect("installs");
    assert!(!telemetry::stderr_line(format_args!("after the sink")));
    assert!(!telemetry::stderr_line(format_args!("and again")));
    assert_eq!(telemetry::unprinted(), 3);
    let found = telemetry::tail(
        &dir,
        sink.keep_files(),
        &Query::last(10).from_target("telemetry.stderr"),
    );
    assert_eq!(found.records.len(), 1, "named once: {found:?}");
    assert_eq!(found.records[0].message, "the error stream refused a line");
    assert!(
        found.records[0]
            .field("error")
            .and_then(telemetry::OwnedValue::as_str)
            .is_some_and(|said| said.contains("Broken pipe")),
        "{found:?}"
    );

    // A sink's one stderr notice is refused too, and says so where the page
    // reads.
    let refusing =
        Sink::with_target(&Config::new(dir.join("other")), Box::new(Refuses)).expect("opens");
    assert_eq!(refusing.emit(&Event::info("t", "lost")), Emitted::Dropped);
    let said = refusing.health().last_error.expect("named");
    assert!(said.contains("no space left on device"), "{said}");
    assert!(
        said.contains("could not be written to stderr"),
        "the closed stream is named beside the failure: {said}"
    );
    assert_eq!(telemetry::unprinted(), 4);
    let _ignored = std::fs::remove_dir_all(&dir);
    println!("CHILD-RAN");
}
