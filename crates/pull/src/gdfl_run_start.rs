//! `pull::gdfl_run::start`: the child's half of the two-phase start (IA-2;
//! round 4, BL-A; D-4849; design §4).
//!
//! A press starts all its kinds or none. On a v4 store the api starts one
//! child per kind; each child plans, takes its locks, takes its id from the
//! runs ledger and writes its claim ([`super::claim_with`]), then prints one
//! line, `claimed run=<id> kind=<k>`, and waits for one line back:
//!
//! | it reads | it does |
//! |---|---|
//! | `go` | runs |
//! | `release`, any other line, or end of input (the api died) | releases |
//! | nothing for [`GO_WAIT`] | releases |
//!
//! A release truncates the claim, removes the request sidecar, releases the
//! flocks and emits the audited lifecycle event "run released before start";
//! the child has read no day and written no journal line, and its id stays in
//! the runs ledger, never given again (D-4847). The api's half (who sends
//! `go`, and the 409 naming each busy kind) is IA-10a's.

use std::io::{BufRead, Write};
use std::sync::mpsc;
use std::time::Duration;

use super::Claimed;
use super::locks::TARGET;
use crate::gdfl_import::ImportKind;

/// How long a claimed child waits for `go` before it releases.
pub const GO_WAIT: Duration = Duration::from_secs(15);

/// What a claimed child heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    /// `go`: run.
    Go,
    /// `release`, another line, or the end of the input.
    Release,
    /// Nothing within the wait.
    Silent,
}

impl Gate {
    /// The word the release event names.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Go => "go",
            Self::Release => "release",
            Self::Silent => "silent",
        }
    }
}

/// The child's two lines with its controller (design §5.1): its stdout and
/// stdin, or a test's.
pub trait StartGate {
    /// Says the run is claimed.
    fn claimed(&mut self, run: u64, kind: ImportKind);
    /// Waits at most `at_most` for the answer.
    fn wait(&mut self, at_most: Duration) -> Gate;
}

/// A child released before `go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Released {
    /// Its run id (left unused in the runs ledger).
    pub run: u64,
    /// Its kind.
    pub kind: ImportKind,
    /// What it heard.
    pub heard: Gate,
    /// A release step that failed, by name.
    pub fault: Option<String>,
}

/// The child's second phase: says `claimed`, waits at most [`GO_WAIT`] for
/// `go`, and on anything else releases everything and answers [`Released`].
///
/// # Errors
///
/// [`Released`] when the answer was not `go`.
pub fn await_go(claimed: Claimed, gate: &mut dyn StartGate) -> Result<Claimed, Released> {
    await_go_within(claimed, gate, GO_WAIT)
}

/// [`await_go`] with the wait given.
///
/// # Errors
///
/// As [`await_go`].
pub fn await_go_within(
    claimed: Claimed,
    gate: &mut dyn StartGate,
    wait: Duration,
) -> Result<Claimed, Released> {
    let (run, kind) = (claimed.run, claimed.kind);
    gate.claimed(run, kind);
    let heard = gate.wait(wait);
    if heard == Gate::Go {
        return Ok(claimed);
    }
    let fault = claimed.release().err();
    let event = telemetry::Event::info(TARGET, "run released before start")
        .with("run", run)
        .with("kind", kind.as_str())
        .with("heard", heard.as_str())
        .with("released", fault.as_deref().unwrap_or("clean"));
    let _dropped_when_filtered = telemetry::emit(&event);
    Err(Released {
        run,
        kind,
        heard,
        fault,
    })
}

/// The api child's gate: its stdout and stdin. The answer is read on a
/// thread of its own, so the wait is bounded whatever the pipe does.
#[derive(Debug, Default)]
pub struct StdioGate;

impl StartGate for StdioGate {
    fn claimed(&mut self, run: u64, kind: ImportKind) {
        let mut out = std::io::stdout().lock();
        // A closed stdout is an api that went: the wait then reads end of
        // input and releases, so a failed write is not a second refusal.
        let _said =
            writeln!(out, "claimed run={run} kind={}", kind.as_str()).and_then(|()| out.flush());
    }

    fn wait(&mut self, at_most: Duration) -> Gate {
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("gdfl-start-gate".to_owned())
            .spawn(move || {
                let mut line = String::new();
                let read = std::io::stdin().lock().read_line(&mut line);
                let _sent = tx.send(match read {
                    Ok(0) | Err(_) => None,
                    Ok(_) => Some(line),
                });
            });
        if spawned.is_err() {
            return Gate::Release;
        }
        answer_of(rx.recv_timeout(at_most))
    }
}

/// The gate a received line (or its absence) means.
fn answer_of(got: Result<Option<String>, mpsc::RecvTimeoutError>) -> Gate {
    match got {
        Ok(Some(line)) if line.trim_end() == "go" => Gate::Go,
        Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => Gate::Release,
        Err(mpsc::RecvTimeoutError::Timeout) => Gate::Silent,
    }
}

#[cfg(test)]
mod answer_tests {
    #![allow(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]
    use super::*;

    /// Only `go` runs; any other line, the end of input and a closed sender
    /// release; nothing in time is silent.
    #[test]
    fn only_go_runs_and_silence_is_named() {
        assert_eq!(answer_of(Ok(Some("go\n".to_owned()))), Gate::Go);
        assert_eq!(answer_of(Ok(Some("release\n".to_owned()))), Gate::Release);
        assert_eq!(answer_of(Ok(Some("gone\n".to_owned()))), Gate::Release);
        assert_eq!(answer_of(Ok(None)), Gate::Release);
        assert_eq!(
            answer_of(Err(mpsc::RecvTimeoutError::Disconnected)),
            Gate::Release
        );
        assert_eq!(
            answer_of(Err(mpsc::RecvTimeoutError::Timeout)),
            Gate::Silent
        );
    }
}
