//! The operator entry point for the HTTP surface.
//!
//! This file holds one call and one event, on purpose. A `main` cannot be
//! entered by a unit test — `cargo test` builds the binary with its own
//! harness and never runs it — so every line of logic left here is a line no
//! test can reach. Everything therefore lives in [`api::server`], which is
//! ordinary library code, and `tests/binary.rs` runs this binary once so that
//! even this call is measured rather than assumed.
//!
//! The event is the second thing, and it is here rather than in the library
//! because *here* is the only place that knows the process is over. Its
//! decision — which level, which reason — is split into [`exit_note`], which
//! is a pure function a unit test in this same file can drive across every
//! code, so no arm of it depends on a child process to be reached.
//!
//! Usage: `api [serve [ADDR] | report]`.

// A BINARY IS ITS OWN CRATE ROOT, which is exactly why this was missing.
// `lib.rs` carries the attribute and it does not reach here: `main.rs` is
// compiled as a separate root, so `crates/api` had one root forbidding unsafe
// and one that did not. CI gate 16 checks every root for this reason — the
// omission is invisible from the library side and costs nothing to state.
#![forbid(unsafe_code)]

/// Serves, or reports, and returns the exit code the operator's shell reads.
///
/// The shutdown signal is passed IN rather than installed inside the server,
/// so that every arm of [`api::server::run`] is drivable from a test without a
/// signal and without a hard kill. Ctrl-C is what an operator has; an
/// already-resolved future is what a test has.
#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = api::server::run(&args, Box::pin(tokio::signal::ctrl_c())).await;
    note_exit(code, args.len());
    std::process::ExitCode::from(code)
}

/// The last thing this process does: say which code the shell is about to
/// read, and what that code means.
///
/// # What was invisible without it
///
/// The exit code is the one fact every operator and every monitor acts on, and
/// it was the one fact that left no durable trace. A binary that starts,
/// refuses its arguments and exits puts `unknown argument` on stderr, and that
/// is gone the moment the terminal closes; a serve that falls over while
/// accepting puts `server stopped: …` there and goes the same way. The only
/// event this binary ever wrote was `api.serve listening`, emitted on the path
/// that had ALREADY bound a port — so a process that never got that far said
/// nothing at all, and the log of a machine that failed to start was
/// byte-for-byte the log of a machine nobody ran.
///
/// # WHICH OF THESE REACH A FILE, AND WHICH CANNOT
///
/// `telemetry::install` happens inside `api::server::run`, on the serve path,
/// after the listener binds. This event is therefore written on some paths and
/// dropped on others, and pretending otherwise would be exactly the silent
/// fallback `CLAUDE.md` §4 bans:
///
/// * **Written** — exit `0` or `1` after a successful bind: a served process
///   stopping, cleanly or on an accept failure. The sink was installed pages
///   earlier, so the line lands beside the `api.serve listening` it closes.
///   These are the two the 64 MiB window exists to keep.
/// * **Dropped** — `MISUSED`, a `FAILED` from a bind that never succeeded, and
///   both codes from `report`. No sink exists yet, so `telemetry::emit`
///   returns `NotInstalled`: nothing is printed, nothing panics, nothing is
///   slowed down, and the reason still reaches stderr the way it always did.
///   Making those durable means installing the sink before the command is
///   parsed, which is a change to `server.rs` and not to this file.
///
/// # Bounded by structure
///
/// Once per process, at the one moment a process has only once. There is no
/// loop here and nothing that grows with the data.
///
/// # What is NOT in the fields
///
/// Not `argv`. Its length grows with whatever the operator typed and its
/// contents are theirs, and a log is a file somebody pastes into an issue
/// (`CLAUDE.md` §8). The COUNT is the part that carries information: it
/// separates "no command at all, so it served" from "a word this build does
/// not know", which is precisely what the exit code alone leaves ambiguous.
///
/// The process id is here because the sink directory is shared — every `api`
/// run on the machine appends to the same file, and a record carries `seq` and
/// a timestamp but no process identity. Without it, two exits an hour apart
/// and two exits from two processes at the same instant read alike.
fn note_exit(code: u8, argc: usize) {
    let (level, said, why) = exit_note(code);
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(level, "api.main", said)
            .with("code", telemetry::Value::Uint(u64::from(code)))
            .with("why", telemetry::Value::Str(why))
            .with("argc", telemetry::Value::Uint(argc as u64))
            .with("pid", telemetry::Value::Uint(u64::from(std::process::id()))),
    );
}

/// The level, the sentence and the reason one exit code has earned.
///
/// Split from [`note_exit`] because it is the only half of this pair a unit
/// test can reach. `cargo test` replaces `main` with its own harness, and
/// `tests/binary.rs` drives three commands — a clean report, a degraded one
/// and a misuse — so a `match` left inline would have arms no test in this
/// repository could enter, and the coverage gate `docs/04-invariants.md` X-06
/// states could never hold them.
///
/// The words are the documented meanings of the constants in `api::server`,
/// taken from there rather than invented, so this line and the definition
/// cannot drift into saying two different things about one number.
fn exit_note(code: u8) -> (telemetry::Level, &'static str, &'static str) {
    match code {
        api::server::OK => (
            telemetry::Level::Info,
            "exited cleanly",
            "everything went as asked",
        ),
        api::server::FAILED => (
            telemetry::Level::Error,
            "exited non-zero",
            "it was asked for something reasonable and could not do it",
        ),
        api::server::MISUSED => (
            telemetry::Level::Error,
            "exited non-zero",
            "it was asked for something it does not understand",
        ),
        // NON-ZERO AND AT ERROR, THOUGH THE WORK COMPLETED. D-0026 exists
        // because a zero here let a monitor read green while one of the two
        // masters had never been opened; a level below `error` would put the
        // same defect back one layer down, in the file the monitor's operator
        // reads afterwards.
        api::server::DEGRADED => (
            telemetry::Level::Error,
            "exited non-zero",
            "it did the work and the answer must not be trusted",
        ),
        // NOT UNREACHABLE, AND NOT A PANIC. `run` returns a `u8`, so a code no
        // constant here names is a thing a later change can produce, and the
        // honest answer is to say the number and admit this build cannot name
        // it — never to claim one of the four meanings above.
        _ => (
            telemetry::Level::Error,
            "exited non-zero",
            "an exit code this build does not name",
        ),
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{exit_note, note_exit};

    /// Every code this build can return says something of its own, and zero is
    /// the only one that is not an error.
    #[test]
    fn each_exit_code_carries_its_own_reason_and_only_zero_is_not_an_error() {
        // The fifth is a code no constant names: the arm that exists so a
        // later `run` cannot return a number this file silently mislabels.
        let codes = [
            api::server::OK,
            api::server::FAILED,
            api::server::MISUSED,
            api::server::DEGRADED,
            250,
        ];
        let mut reasons: Vec<&str> = Vec::new();
        for code in codes {
            let (level, said, why) = exit_note(code);
            assert_eq!(
                level == telemetry::Level::Info,
                code == api::server::OK,
                "only a zero exit is not an error, and {code} disagreed"
            );
            assert_eq!(
                said == "exited cleanly",
                code == api::server::OK,
                "the sentence and the level must agree about {code}"
            );
            assert!(
                !reasons.contains(&why),
                "{code} reuses another code's reason: {why}"
            );
            reasons.push(why);
        }
        assert_eq!(reasons.len(), codes.len(), "five codes, five reasons");
    }

    /// THE EVENT IS WRITTEN, not merely built.
    ///
    /// Installs a sink over a temporary directory and reads the line back
    /// through the public reader, so that deleting the `emit` in [`note_exit`]
    /// fails a test instead of quietly removing the only durable record of how
    /// a process ended. This is the one test here that touches the process-wide
    /// global — `OnceLock` is per process, so there can only be one.
    #[test]
    fn the_exit_event_reaches_the_log_carrying_the_code_and_the_reason() {
        let dir = std::env::temp_dir().join(format!("brutex-api-main-{}", std::process::id()));
        let _ignored = std::fs::remove_dir_all(&dir);
        let sink = telemetry::install(&telemetry::Config::new(&dir)).expect("installs");

        note_exit(api::server::MISUSED, 1);

        let found = telemetry::tail(&dir, sink.keep_files(), &telemetry::Query::last(10));
        assert_eq!(found.records.len(), 1, "one exit, one line");
        let line = &found.records[0];
        assert_eq!(line.target, "api.main");
        assert_eq!(line.level, telemetry::Level::Error);
        assert_eq!(line.message, "exited non-zero");
        assert_eq!(
            line.field("code").and_then(telemetry::OwnedValue::as_u64),
            Some(u64::from(api::server::MISUSED))
        );
        assert_eq!(
            line.field("argc").and_then(telemetry::OwnedValue::as_u64),
            Some(1)
        );
        assert_eq!(
            line.field("pid").and_then(telemetry::OwnedValue::as_u64),
            Some(u64::from(std::process::id()))
        );
        assert!(
            line.field("why")
                .and_then(telemetry::OwnedValue::as_str)
                .is_some_and(|why| why.contains("does not understand")),
            "the reason travels with the code: {:?}",
            line.fields
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }
}
