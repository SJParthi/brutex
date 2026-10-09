//! One line on the process's stderr that never panics, and whose failure is
//! never hidden.
//!
//! # Why this exists (r53-1, D-4413)
//!
//! `eprintln!` panics when stderr is gone — "failed printing to stderr: Broken
//! pipe (os error 32)" — and the release profile sets `panic = "abort"`, so a
//! diagnostic line about a degraded decode killed a long backfill with signal 6
//! when its stderr was a pipe whose reader had gone away. Measured by the r53
//! audit on `pull::csv::decode` with stderr connected to a closed pipe. A line
//! that describes the work must never be what stops it.
//!
//! # Why a failed line is still recorded
//!
//! `CLAUDE.md` §4 forbids a fallback that hides a failure. Every production
//! stderr line in `pull` and in this crate is written beside an event that
//! carries the same fact, so the fact itself survives a closed stderr. What the
//! closed stream loses is the operator's terminal copy, and that loss is
//! recorded twice: [`unprinted`] counts every line that could not be written,
//! and the FIRST failure in the process is written to the installed sink as a
//! `telemetry.stderr` `Warn` naming the error — once, because a stderr that is
//! gone stays gone, and one event per line would turn one fault into a flood.
//!
//! # Cost
//!
//! One locked write and one flush of stderr, as `eprintln!` costs. On failure,
//! one relaxed atomic add and, the first time only, one event.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::event::Event;
use crate::value::Value;

/// Lines this process could not write to stderr.
static UNPRINTED: AtomicU64 = AtomicU64::new(0);

/// Whether the first failure has been written to the log.
static NAMED: AtomicBool = AtomicBool::new(false);

/// Writes `line` and a newline to this process's stderr, and answers whether
/// it was written.
///
/// Never panics. A failure is counted in [`unprinted`], and the first one in
/// the process is written to the installed sink; see the module header.
#[must_use = "a line the stream refused is counted; say why it may be ignored"]
pub fn stderr_line(line: core::fmt::Arguments<'_>) -> bool {
    noted(line_to(&mut std::io::stderr().lock(), line))
}

/// How many lines this process could not write to stderr.
#[must_use]
pub fn unprinted() -> u64 {
    UNPRINTED.load(Ordering::Relaxed)
}

/// The write, apart from the stream, so a test can hand it one that refuses.
fn line_to(out: &mut impl std::io::Write, line: core::fmt::Arguments<'_>) -> std::io::Result<()> {
    writeln!(out, "{line}").and_then(|()| out.flush())
}

/// Counts a failed line and names the first one in the log.
fn noted(outcome: std::io::Result<()>) -> bool {
    let Err(e) = outcome else {
        return true;
    };
    UNPRINTED.fetch_add(1, Ordering::Relaxed);
    // NAMED ONLY ONCE IT LANDED. A failure before a sink is installed, or one
    // the sink itself dropped, leaves the next failure to try again — the
    // record is what the flag stands for, not the attempt.
    if !NAMED.swap(true, Ordering::AcqRel) {
        let error = e.to_string();
        let landed = crate::emit(
            &Event::warn("telemetry.stderr", "the error stream refused a line")
                .with("error", Value::Str(&error))
                .with(
                    "why",
                    Value::Str(
                        "this line and every later stderr line of this process are lost \
                         there; each one's fact is in its own event, and \
                         telemetry::unprinted() counts the lines",
                    ),
                ),
        );
        if !landed.is_written() {
            NAMED.store(false, Ordering::Release);
        }
    }
    false
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
    use super::{line_to, noted, unprinted};

    /// A stream that refuses every write, as a closed pipe does.
    struct Closed;

    impl std::io::Write for Closed {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// A stream that takes the bytes and refuses the flush.
    struct Unflushable(Vec<u8>);

    impl std::io::Write for Unflushable {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::Error::other("flush refused"))
        }
    }

    /// The line and its newline reach the stream; a refused write and a
    /// refused flush are both failures, and each is counted — never a panic.
    #[test]
    fn a_line_is_written_whole_and_a_refusal_is_counted_not_panicked() {
        let mut taken = Vec::new();
        assert!(line_to(&mut taken, format_args!("one {}", 2)).is_ok());
        assert_eq!(taken, b"one 2\n");

        let before = unprinted();
        assert!(!noted(line_to(&mut Closed, format_args!("lost"))));
        let mut half = Unflushable(Vec::new());
        assert!(!noted(line_to(&mut half, format_args!("kept"))));
        assert_eq!(half.0, b"kept\n");
        assert!(
            unprinted() >= before + 2,
            "both refusals counted (other tests may add their own)"
        );
        assert!(noted(Ok(())), "a written line is no failure");
    }
}
