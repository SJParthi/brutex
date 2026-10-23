//! THE MEASUREMENT behind [`super::APPEND_LOCK_WAIT`]'s row in
//! `docs/06-limits.md` (AG-06): `open_run`, `append_member` and `close_run`
//! latency with 1, 2 and 8 writers at once, 200 samples of each operation in
//! each, over a journal the caller CLONED (`cp -c`) from the operator's, so
//! the index catch-up and the file sizes are the real ones and the real
//! journal is never written. Uses the public journal API only, so the same
//! file runs on the `try_lock` build for the "before" column.
//!
//! Run by hand: `BRUTEX_AG06_MEASURE_ROOT=<root whose audit/ holds the clone>
//! cargo test -p api --locked --lib audit::lock_wait_measure -- --ignored
//! --nocapture`.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::print_stdout
)]

use super::*;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

/// Nearest-rank percentile of sorted microseconds.
fn rank(sorted: &[u128], p: usize) -> u128 {
    let n = sorted.len();
    let at = (n * p).div_ceil(100).max(1) - 1;
    sorted[at.min(n - 1)]
}

#[test]
#[ignore = "a measurement over a cloned journal, run by hand with BRUTEX_AG06_MEASURE_ROOT"]
fn measure_append_latency_under_contention() {
    let root = std::env::var("BRUTEX_AG06_MEASURE_ROOT")
        .expect("BRUTEX_AG06_MEASURE_ROOT names a store root holding a CLONE of the journal");
    let journal = Arc::new(Journal::at(Path::new(&root)));
    let day = pull::session::Day::new(2025, 7, 1).expect("a date");
    let window = Window::new(day, day).expect("forwards");
    let at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_750_000_000);
    println!("journal records before: {}", journal.look().records());
    for writers in [1_usize, 2, 8] {
        let per = 200 / writers;
        let barrier = Arc::new(Barrier::new(writers));
        let samples: Vec<Vec<(usize, u128, bool)>> = (0..writers)
            .map(|w| {
                let (journal, barrier) = (Arc::clone(&journal), Arc::clone(&barrier));
                std::thread::spawn(move || {
                    let mut out = Vec::with_capacity(per * 3);
                    barrier.wait();
                    for i in 0..per {
                        let src = format!("ag06-measure-w{writers}-{w}-{i}");
                        let head = Record::opened(Scope::Spot, at, &src, window, 1, 0xa5);
                        let t = Instant::now();
                        let thread = journal.open_run(&head);
                        out.push((0, t.elapsed().as_micros(), thread.is_ok()));
                        let failure =
                            Record::member_failure(Scope::Spot, at, &src, window, "measured");
                        let t = Instant::now();
                        let member = journal.append_member(&failure, "measured");
                        out.push((1, t.elapsed().as_micros(), member.is_ok()));
                        let close = Record::refused(Scope::Spot, Outcome::Stored, at, &src, "done");
                        let t = Instant::now();
                        let closed = match thread {
                            Ok(mut thread) => journal.close_run(&mut thread, &close),
                            Err(_) => journal.close_unopened(&head, &close),
                        };
                        out.push((2, t.elapsed().as_micros(), closed.is_ok()));
                    }
                    out
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().expect("a writer"))
            .collect();
        for (op, name) in ["open_run", "append_member", "close_run"]
            .iter()
            .enumerate()
        {
            let mut micros: Vec<u128> = Vec::new();
            let mut refused = 0_usize;
            for &(o, us, ok) in samples.iter().flatten() {
                if o == op {
                    micros.push(us);
                    refused += usize::from(!ok);
                }
            }
            micros.sort_unstable();
            println!(
                "AG06 writers={writers} op={name} n={} refused={refused} p50_us={} p99_us={} max_us={}",
                micros.len(),
                rank(&micros, 50),
                rank(&micros, 99),
                micros.last().copied().unwrap_or(0),
            );
        }
    }
    println!("journal records after: {}", journal.look().records());
}
