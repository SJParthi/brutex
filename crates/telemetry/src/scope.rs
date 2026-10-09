//! Which run the work being done RIGHT NOW belongs to — per thread, scoped to
//! a future or a block, with no runtime dependency.
//!
//! # What this replaces, and why the ambient key was not enough (sobs-14)
//!
//! [`crate::Sink::claim_run`] holds ONE run key for the whole process, and
//! [`crate::Sink::emit`] stamps every event with it. That is right for a
//! process doing one thing. A server is not one: while a pull run held the
//! key, every unrelated event the process wrote — a page load's
//! `api.request`, an `autopilot` pause, a sweep's refusal — was filed under
//! that pull, so `/logs?run=` returned a story with other stories spliced in.
//! Measured on the audit's probe P9: a claimed run 777 answered
//! `["autopilot","api.request","pull.run"]`.
//!
//! # The repair, and why it needs no runtime
//!
//! `sink.rs` rejected a task-local because it means a runtime dependency, and
//! that stands: this crate depends on nothing. What a task-local does can be
//! done with `std` alone. A future's code only ever runs inside its `poll`, so
//! [`in_run`] sets a THREAD-local for the duration of each `poll` and restores
//! the previous value when the `poll` returns or unwinds. Whatever thread
//! polls it, whenever, the code inside sees its own run and nothing else does:
//! another task polled on the same thread a moment later sees the restored
//! value. [`enter`] does the same for a synchronous block, which is how work
//! handed to another thread carries the run with it.
//!
//! A scope OUTRANKS the ambient key in [`crate::Sink::emit`]; an event written
//! outside every scope still carries the ambient key, so a single-run process
//! that never opens a scope is unchanged. An explicit
//! [`crate::Sink::emit_for_run`] outranks both.
//!
//! # What a scope does not follow
//!
//! A thread-local does not cross a spawn. A future spawned from inside a scope
//! runs outside it unless the spawner wraps it, which [`inherit`] does in one
//! call. That is the honest limit of a per-thread value, and the caller that
//! spawns is the only place that can know the spawned work is part of the run.
//!
//! # Cost
//!
//! [`current_run`]: one thread-local read. [`in_run`]: one allocation per
//! scope (the wrapped future is boxed so it can be pinned without `unsafe`,
//! which this crate forbids), then per `poll` two thread-local writes. Neither
//! grows with anything. Not timed: **UNVERIFIED as a measurement**, argued
//! from the shape of the code. `CLAUDE.md` §3 rule 6.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::cell::Cell;

thread_local! {
    /// The innermost open scope's run on this thread, or zero for none.
    ///
    /// Zero is the absence of a run everywhere in this crate, so a scope of
    /// zero is the same as no scope: the ambient key decides.
    static CURRENT: Cell<u64> = const { Cell::new(0) };
}

/// The run the innermost scope open on this thread names, if any.
///
/// [`None`] outside every scope, inside a scope of zero, and on a thread
/// whose thread-locals are already being torn down.
#[must_use]
pub fn current_run() -> Option<u64> {
    CURRENT.try_with(Cell::get).ok().filter(|run| *run != 0)
}

/// An open scope on this thread. Dropping it restores what it replaced.
///
/// Restored on unwind too, because `Drop` runs then: a panic inside a scope
/// cannot leave the thread stamping later work with a run that has ended.
#[must_use = "the scope closes when this is dropped"]
#[derive(Debug)]
pub struct Entered {
    /// What the thread named before this scope opened.
    previous: u64,
}

impl Drop for Entered {
    fn drop(&mut self) {
        let previous = self.previous;
        let _torn_down = CURRENT.try_with(|current| current.set(previous));
    }
}

/// Opens a scope naming `run` on this thread until the guard is dropped.
///
/// For synchronous work, and for the first line of a closure handed to
/// another thread with the run [`current_run`] gave on the thread that
/// handed it over.
pub fn enter(run: u64) -> Entered {
    Entered {
        previous: CURRENT
            .try_with(|current| current.replace(run))
            .unwrap_or(0),
    }
}

/// A future whose every `poll` runs inside a scope naming one run.
#[must_use = "a future does nothing unless it is polled"]
pub struct InRun<F> {
    /// The run every event written while polling `inner` carries.
    run: u64,
    /// Boxed so it can be pinned without `unsafe` structural projection.
    inner: Pin<Box<F>>,
}

impl<F> core::fmt::Debug for InRun<F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InRun")
            .field("run", &self.run)
            .finish_non_exhaustive()
    }
}

impl<F: Future> Future for InRun<F> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        let _scope = enter(self.run);
        self.inner.as_mut().poll(cx)
    }
}

/// Runs `future` inside a scope naming `run`, on whichever thread polls it.
pub fn in_run<F: Future>(run: u64, future: F) -> InRun<F> {
    InRun {
        run,
        inner: Box::pin(future),
    }
}

/// Runs `future` inside the scope open where THIS call is made.
///
/// The one call a spawner needs so that spawned work stays part of the run
/// that spawned it. Outside every scope it opens a scope of zero, which is no
/// scope at all.
pub fn inherit<F: Future>(future: F) -> InRun<F> {
    in_run(current_run().unwrap_or(0), future)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{current_run, enter, in_run, inherit};
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll, Waker};

    /// Polls a future to completion on this thread with a no-op waker. Every
    /// future here is ready or yields a bounded number of times.
    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = Box::pin(future);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(out) = future.as_mut().poll(&mut cx) {
                return out;
            }
        }
    }

    /// A future that is pending once, then ready with what the scope said on
    /// each of its two polls.
    struct TwoPolls {
        seen: Vec<Option<u64>>,
    }

    impl Future for TwoPolls {
        type Output = Vec<Option<u64>>;
        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            self.seen.push(current_run());
            if self.seen.len() < 2 {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            Poll::Ready(core::mem::take(&mut self.seen))
        }
    }

    #[test]
    fn a_scope_names_its_run_only_while_it_is_polled() {
        assert_eq!(current_run(), None, "a fresh thread is in no scope");
        let mut scoped = Box::pin(in_run(777, TwoPolls { seen: Vec::new() }));
        let mut cx = Context::from_waker(Waker::noop());
        assert!(scoped.as_mut().poll(&mut cx).is_pending());
        assert_eq!(current_run(), None, "between polls the thread is clean");
        let Poll::Ready(seen) = scoped.as_mut().poll(&mut cx) else {
            panic!("two polls finish it");
        };
        assert_eq!(seen, [Some(777), Some(777)], "inside, every poll sees 777");
        assert_eq!(current_run(), None);
    }

    #[test]
    fn scopes_nest_and_a_zero_scope_is_no_scope() {
        let outer = enter(5);
        assert_eq!(current_run(), Some(5));
        {
            let _inner = enter(9);
            assert_eq!(current_run(), Some(9), "the innermost wins");
        }
        assert_eq!(current_run(), Some(5), "closing it restores the outer");
        {
            let _none = enter(0);
            assert_eq!(current_run(), None, "zero names no run");
        }
        drop(outer);
        assert_eq!(current_run(), None);
    }

    #[test]
    fn a_panic_inside_a_scope_does_not_leave_the_thread_in_it() {
        let unwound = std::panic::catch_unwind(|| {
            let _scope = enter(41);
            panic!("inside the scope");
        });
        assert!(unwound.is_err());
        assert_eq!(current_run(), None, "the guard restored on unwind");
    }

    #[test]
    fn inherit_carries_the_run_open_where_it_is_called() {
        let carried = {
            let _scope = enter(12);
            inherit(async { current_run() })
        };
        assert_eq!(current_run(), None);
        assert_eq!(
            block_on(carried),
            Some(12),
            "the run travels with the future"
        );
        assert_eq!(block_on(inherit(async { current_run() })), None);
        assert!(format!("{:?}", in_run(3, async {})).contains("run: 3"));
    }

    /// THE AUDIT'S PROBE P9, REPLAYED THROUGH A SINK (sobs-14, D-4451).
    ///
    /// A pull's events are written inside its scope; a page load and an
    /// autopilot pause are written outside it, on this thread between polls
    /// and on another thread while the scope is open. Filtering by the pull's
    /// run returns the pull's events and nothing else. And the ambient key a
    /// single-run process claims still stamps work outside every scope, while
    /// a scope inside it outranks it.
    #[test]
    fn a_run_filter_returns_only_the_runs_own_events() {
        use crate::{Event, Query, Sink, sink::Config, tail};
        let dir =
            std::env::temp_dir().join(format!("brutex-telemetry-{}-scope-p9", std::process::id()));
        let _ignored = std::fs::remove_dir_all(&dir);
        let sink = std::sync::Arc::new(Sink::open(&Config::new(&dir)).expect("a sink"));
        let mut pull = Box::pin(in_run(777, {
            let sink = std::sync::Arc::clone(&sink);
            async move {
                assert!(sink.emit(&Event::info("pull.run", "started")).is_written());
                YieldOnce(false).await;
                assert!(
                    sink.emit(&Event::info("pull.member", "landed"))
                        .is_written()
                );
            }
        }));
        let mut cx = Context::from_waker(Waker::noop());
        assert!(pull.as_mut().poll(&mut cx).is_pending());
        // BETWEEN POLLS, ON THIS THREAD: another task's request line.
        assert!(
            sink.emit(&Event::warn("api.request", "served"))
                .is_written()
        );
        // AND ON ANOTHER THREAD WHILE A SCOPE IS OPEN HERE.
        {
            let _open = enter(777);
            let other = std::sync::Arc::clone(&sink);
            std::thread::spawn(move || {
                other.emit(&Event::warn("autopilot", "paused")).is_written()
            })
            .join()
            .map(|written| assert!(written))
            .expect("joined");
        }
        assert!(pull.as_mut().poll(&mut cx).is_ready());

        let targets = |run| -> Vec<String> {
            tail(
                &dir,
                sink.keep_files(),
                &Query::last(crate::MAX_LIMIT).from_run(run),
            )
            .records
            .into_iter()
            .map(|record| record.target)
            .collect()
        };
        assert_eq!(
            targets(777),
            ["pull.member", "pull.run"],
            "newest first, only the pull's"
        );
        assert_eq!(
            targets(0),
            ["autopilot", "api.request"],
            "the rest carry no run"
        );

        // THE AMBIENT KEY, UNCHANGED OUTSIDE A SCOPE AND OUTRANKED INSIDE ONE.
        assert!(sink.claim_run(5));
        assert!(
            sink.emit(&Event::info("cli.lifecycle", "ambient"))
                .is_written()
        );
        {
            let _scope = enter(6);
            assert!(
                sink.emit(&Event::info("cli.lifecycle", "scoped"))
                    .is_written()
            );
            assert!(
                sink.emit_for_run(8, &Event::info("cli.lifecycle", "explicit"))
                    .is_written(),
                "an explicit run outranks the scope"
            );
        }
        sink.release_run(5);
        assert_eq!(targets(5).len(), 1, "the ambient key outside a scope");
        assert_eq!(targets(6).len(), 1, "the scope inside the claim");
        assert_eq!(targets(8).len(), 1, "the explicit run");
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// Pending once, then ready.
    struct YieldOnce(bool);

    impl Future for YieldOnce {
        type Output = ();
        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if self.0 {
                return Poll::Ready(());
            }
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }

    /// The scope is per THREAD: another thread writing while a scope is open
    /// here sees none of it.
    #[test]
    fn another_thread_never_sees_this_threads_scope() {
        let _scope = enter(88);
        let there = std::thread::spawn(current_run).join().expect("joined");
        assert_eq!(there, None);
        let carried = std::thread::spawn({
            let run = current_run().unwrap_or(0);
            move || {
                let _scope = enter(run);
                current_run()
            }
        })
        .join()
        .expect("joined");
        assert_eq!(carried, Some(88), "handed over explicitly, it is carried");
    }
}
