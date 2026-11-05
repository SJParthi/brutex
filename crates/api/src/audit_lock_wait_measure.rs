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

/// Nearest-rank percentile of sorted microseconds; `None` for no sample.
fn rank(sorted: &[u128], p: usize) -> Option<u128> {
    let at = (sorted.len() * p).div_ceil(100).max(1) - 1;
    sorted.get(at).copied()
}

/// A rank as the printed line shows it: the microseconds, or `none`.
fn shown(rank: Option<u128>) -> String {
    rank.map_or_else(|| "none".to_owned(), |micros| micros.to_string())
}

#[test]
#[ignore = "a measurement over a cloned journal, run by hand with BRUTEX_AG06_MEASURE_ROOT"]
fn measure_append_latency_under_contention() {
    let root = std::env::var("BRUTEX_AG06_MEASURE_ROOT")
        .expect("BRUTEX_AG06_MEASURE_ROOT names a store root holding a CLONE of the journal");
    let _measured = latency_over(Path::new(&root), 200);
}

/// The measurement's own path at small sizes (BASE-COV-API, D-7344): every
/// operation it asks is counted and over a journal it can write none is
/// refused. Extremes (repair 3, D-8660): eight samples, the fewest that give
/// each of eight writers one; nine, which two and eight writers cannot share
/// evenly; sixteen; seven, which leaves the eight writers none, an empty
/// spread printed as none; two, which leaves them none and the two writers
/// one each; one, which only the lone writer runs; a journal that already
/// holds what an earlier measurement wrote; a root whose audit folder cannot
/// be made (a file stands there), every operation refused and counted; and,
/// at once, one sample over that root, and seven over a journal that already
/// holds records; out of the box, no sample at all: nothing asked, nothing
/// refused, nothing written.
#[test]
fn the_lock_wait_measurement_runs_at_a_small_size() {
    let root = crate::scratch::path("ag06-measure-small");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    let mut written = 0;
    // (samples, operations asked, over the shared journal): 3 a sample, each
    // writer count running `samples / writers` samples on each of its
    // writers. The shared journal is new at the first row and holds every
    // shared row's records after it; every other row has a journal of its
    // own. FOUR RECORDS A SAMPLE: the opening, the member failure, and the
    // close, which writes the opening record again before it because the
    // member stands between them.
    for (at, (samples, asked, shared)) in [
        (8, 72, true),
        (9, 75, true),
        (16, 144, false),
        (7, 39, false),
        (2, 12, false),
        (1, 3, false),
        (7, 39, true),
    ]
    .into_iter()
    .enumerate()
    {
        let own = crate::scratch::path(&format!("ag06-measure-small-{at}"));
        let _ = std::fs::remove_dir_all(&own);
        std::fs::create_dir_all(&own).expect("a scratch root");
        let measured = latency_over(if shared { &root } else { &own }, samples);
        let before = if shared { written } else { 0 };
        assert_eq!(measured.asked, asked, "{samples} samples");
        assert_eq!(measured.refused, 0, "{samples} samples");
        assert_eq!(
            measured.before, before,
            "{samples}: what was written before"
        );
        assert_eq!(
            measured.after,
            before + 4 * u64::try_from(asked / 3).unwrap(),
            "{samples}: four records a sample"
        );
        if shared {
            written = measured.after;
        }
        let _ = std::fs::remove_dir_all(&own);
    }
    let blocked = crate::scratch::path("ag06-measure-blocked");
    let _ = std::fs::remove_dir_all(&blocked);
    std::fs::write(&blocked, b"a file where the root's folders go").expect("a blocking file");
    for (samples, asked) in [(8, 72), (1, 3)] {
        let measured = latency_over(&blocked, samples);
        assert_eq!(
            (measured.asked, measured.refused),
            (asked, asked),
            "{samples} samples over a root nothing can be written under"
        );
        assert_eq!((measured.before, measured.after), (0, 0));
    }
    let nothing = latency_over(&root, 0);
    assert_eq!(
        (
            nothing.asked,
            nothing.refused,
            nothing.before,
            nothing.after
        ),
        (0, 0, written, written),
        "no sample asks nothing and writes nothing"
    );
    assert_eq!(shown(rank(&[], 99)), "none");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_file(&blocked);
}

/// What one measurement asked and found.
struct Measured {
    /// Operations asked, over every writer count.
    asked: usize,
    /// Of them, refused.
    refused: usize,
    /// The journal's records before the first operation.
    before: u64,
    /// And after the last.
    after: u64,
}

/// `samples` of each operation with 1, 2 and 8 writers at once over the
/// journal under `root`, printed; how many operations were asked and refused,
/// and the journal's records before and after.
fn latency_over(root: &Path, samples: usize) -> Measured {
    let journal = Arc::new(Journal::at(root));
    let day = pull::session::Day::new(2025, 7, 1).expect("a date");
    let window = Window::new(day, day).expect("forwards");
    let at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_750_000_000);
    let before = journal.look().records();
    println!("journal records before: {before}");
    let mut all_refused = 0_usize;
    let mut all_asked = 0_usize;
    for writers in [1_usize, 2, 8] {
        let per = samples / writers;
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
            all_refused += refused;
            all_asked += micros.len();
            println!(
                "AG06 writers={writers} op={name} n={} refused={refused} p50_us={} p99_us={} max_us={}",
                micros.len(),
                shown(rank(&micros, 50)),
                shown(rank(&micros, 99)),
                shown(micros.last().copied()),
            );
        }
    }
    let after = journal.look().records();
    println!("journal records after: {after}");
    Measured {
        asked: all_asked,
        refused: all_refused,
        before,
        after,
    }
}
