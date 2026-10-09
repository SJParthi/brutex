//! The one record a crash leaves in the event log (sobs-1, D-4464).
//!
//! Both binaries abort on a panic (`[profile.release] panic = "abort"`), and
//! until this module the panic's message and location went only to stderr —
//! the terminal that is gone by the time anyone diagnoses the crash — while
//! the log file `/logs` reads said nothing at all. [`install`] puts one hook
//! in front of the default one: it writes ONE `error` event, `process
//! panicked`, with the message, the source location and the thread, through
//! the installed sink and syncs it, then hands the panic to the default hook,
//! which prints to stderr exactly as before.
//!
//! # What the hook may not do
//!
//! * **Panic.** Every step returns a value that is dropped, never unwrapped.
//! * **Hang the process it should end.** The sink holds its lock while it
//!   writes, so a panic raised inside the sink would block a hook that emitted
//!   on the panicking thread forever. The event is written from a helper
//!   thread instead, waited for at most [`WAIT`]; past that the hook carries on
//!   without it. [`deliver`] is that bound, and a test holds it.
//! * **Recurse.** A panic raised by the hook's own work on the same thread
//!   writes nothing more.
//! * **Need a sink.** With none installed, a full disk or an unwritable log,
//!   the emit and the barrier return their refusal, which is dropped; the
//!   default hook still prints, so the crash is never silent on stderr either.
//!
//! # Cost
//!
//! Nothing until a panic, then one thread, one event and one barrier, once per
//! panicking thread. No loop, nothing that grows with the data.
//!
//! `tests/panic_hook.rs` runs a real panic under the hook in a child process
//! whose stderr is a closed pipe, with and without a sink.

use std::cell::Cell;
use std::time::Duration;

/// How long the hook waits for its event before handing the panic on.
pub const WAIT: Duration = Duration::from_secs(2);

std::thread_local! {
    static IN_HOOK: Cell<bool> = const { Cell::new(false) };
}

/// Puts the event in front of whatever hook is installed now. `target` names
/// the binary, `cli.main` or `api.main`. Call once, first thing in `main`.
pub fn install(target: &'static str) {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        record(target, info);
        default(info);
    }));
}

/// Writes the panic's one event, bounded, never recursing.
fn record(target: &'static str, info: &std::panic::PanicHookInfo<'_>) {
    if IN_HOOK.with(|inside| inside.replace(true)) {
        return;
    }
    let message = info
        .payload_as_str()
        .unwrap_or("a panic whose payload is not text")
        .to_owned();
    let location = info.location().map_or_else(
        || "unknown".to_owned(),
        |at| format!("{}:{}:{}", at.file(), at.line(), at.column()),
    );
    let thread = std::thread::current()
        .name()
        .unwrap_or("unnamed")
        .to_owned();
    let _landed = deliver(
        move || {
            let _written = telemetry::emit(
                &telemetry::Event::error(target, "process panicked")
                    .with("message", message.as_str())
                    .with("location", location.as_str())
                    .with("thread", thread.as_str())
                    .with("pid", u64::from(std::process::id())),
            );
            if let Some(sink) = telemetry::global() {
                let _synced = sink.sync();
            }
        },
        WAIT,
    );
    IN_HOOK.with(|inside| inside.set(false));
}

/// Runs `write` on its own thread and waits for it at most `wait`. Returns
/// whether it finished in time; a thread that could not be started, a write
/// that panicked and one still blocked all return `false`.
pub(crate) fn deliver(write: impl FnOnce() + Send + 'static, wait: Duration) -> bool {
    let (done, finished) = std::sync::mpsc::channel();
    let started = std::thread::Builder::new()
        .name("panic-log".to_owned())
        .spawn(move || {
            write();
            let _told = done.send(());
        });
    started.is_ok() && finished.recv_timeout(wait).is_ok()
}

#[cfg(test)]
#[allow(
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::deliver;
    use std::time::{Duration, Instant};

    /// A write that finishes is reported; one that is blocked is abandoned at
    /// the bound, not waited for; one that panics is reported as not landed.
    #[test]
    fn delivery_is_bounded_and_never_waits_on_a_blocked_write() {
        assert!(deliver(|| {}, Duration::from_secs(5)));

        let (_hold, blocked) = std::sync::mpsc::channel::<()>();
        let began = Instant::now();
        assert!(!deliver(
            move || {
                let _never = blocked.recv();
            },
            Duration::from_millis(50),
        ));
        assert!(
            began.elapsed() < Duration::from_secs(5),
            "the hook waited {:?} on a blocked write",
            began.elapsed()
        );

        assert!(!deliver(
            || panic!("a write that fails inside the helper"),
            Duration::from_secs(5)
        ));
    }
}
