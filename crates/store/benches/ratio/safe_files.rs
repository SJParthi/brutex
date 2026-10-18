//! C-SF-01, C-SF-02 and C-SF-03: the two shared file doors of R3-16, timed
//! where the store lives, run as a module of `crates/store/benches/ratio.rs`.
//!
//! The directory is `CARGO_TARGET_TMPDIR`, under the target directory, so a
//! run whose target directory is on the store's drive times that drive's
//! calls; the path is printed. Every byte written is invented.
//!
//! * **C-SF-01** `store::safe_open::open_read_len` against a bare
//!   `File::open` of the same regular file, and the one `fstat` the door adds
//!   timed alone: p50, p99 and max at n = 10,000 each. Printed, never gated:
//!   a single call's latency is the scheduler's as much as the code's.
//! * **C-SF-02** the door's cost is flat in the file's size (0 bytes against
//!   4 MiB) and in its directory (alone against beside 2,000 names): the
//!   minimum over [`super::TRIALS`], held to the ceiling.
//! * **C-SF-03** `store::staging::create` on a free name, and over a dead
//!   writer's leftover of 0 bytes and of 4 MiB, alone and beside 2,000 names
//!   (verify-IA-5-r2, follow-up 2): p50, p99 and max (n = 10,000, and 200 at
//!   4 MiB, whose leftovers are written outside the timer). Held to the
//!   ceiling: the 0-byte leftover's p50 alone against beside 2,000 names (the
//!   helper lists no directory). PRINTED, NOT HELD: the 4 MiB leftover
//!   against the 0-byte one. The helper reads no byte of a leftover, but the
//!   host's `unlink` frees its blocks, and that grows with what a dead writer
//!   left: measured 22x to 30x at 4 MiB on `WD_BLACK` (14 Oct 2026), named
//!   `docs/06-limits.md` L-R316-06. A ratio held to the ceiling there would
//!   claim a flatness the host does not give.
//! * **C-SF-04** `store::safe_open::refuse_linked_store_root`, the question
//!   every writing verb asks once: a store with its own `bars` (one `lstat`)
//!   and one whose `bars` is a link (one `lstat` and one `readlink`), p50,
//!   p99 and max at n = 10,000. Printed, never gated.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use store::safe_open;
use store::staging;

/// Single calls timed per spread.
const N: u32 = 10_000;
/// Creates over a 4 MiB leftover: each leftover is 4 MiB written first.
const N_LARGE: u32 = 200;
/// The large file and leftover.
const LARGE: usize = 4 << 20;
/// Names beside the timed one in the crowded directory.
const CROWD: usize = 2_000;

/// A fresh directory of this bench's own under the target's temporary
/// directory.
fn bench_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("brutex-bench-sf-{name}-{}", std::process::id()));
    let _ignored = fs::remove_dir_all(&dir);
    if let Err(e) = fs::create_dir_all(&dir) {
        super::refuse(&format!(
            "the safe-files bench directory would not create: {e}"
        ));
    }
    dir
}

/// `dir` with [`CROWD`] other names in it.
fn crowd(dir: &Path) {
    for i in 0..CROWD {
        if let Err(e) = fs::write(dir.join(format!("zz-other-{i:04}")), b"") {
            super::refuse(&format!("the crowd would not write: {e}"));
        }
    }
}

/// Nanoseconds of each of `n` calls, sorted, with `prepare` outside the
/// timer, printed as p50, p99 and max; returns the p50.
fn spread_ns<T>(
    label: &str,
    n: u32,
    mut prepare: impl FnMut(u32) -> T,
    mut op: impl FnMut(T),
) -> u128 {
    let mut ns: Vec<u128> = (0..n)
        .map(|i| {
            let input = prepare(i);
            let start = Instant::now();
            op(input);
            start.elapsed().as_nanos()
        })
        .collect();
    ns.sort_unstable();
    let at = |q: usize| {
        ns.get(ns.len().saturating_sub(1) * q / 100)
            .copied()
            .unwrap_or(0)
    };
    println!(
        "  {label:<56} p50 {:>9} ns  p99 {:>9} ns  max {:>10} ns  n={n}",
        at(50),
        at(99),
        ns.last().copied().unwrap_or(0)
    );
    at(50)
}

/// C-SF-01 and C-SF-02.
fn reading() -> bool {
    let quiet = bench_dir("read-quiet");
    let crowded = bench_dir("read-crowded");
    crowd(&crowded);
    let mut ok = true;
    for (within, label) in [(&quiet, "alone"), (&crowded, "beside 2,000 names")] {
        let small = within.join("zz-small.bin");
        let large = within.join("zz-large.bin");
        if fs::write(&small, b"")
            .and_then(|()| fs::write(&large, vec![0x5A; LARGE]))
            .is_err()
        {
            super::refuse("the read bench's files would not write");
        }
        println!("C-SF-01 the reading door, {label}: {}", within.display());
        let plain_open = spread_ns(
            "bare File::open, 0 B",
            N,
            |_| (),
            |()| {
                std::hint::black_box(fs::File::open(&small).ok());
            },
        );
        let door = spread_ns(
            "safe_open::open_read_len, 0 B",
            N,
            |_| (),
            |()| {
                std::hint::black_box(safe_open::open_read_len(&small).ok());
            },
        );
        let held = fs::File::open(&small).ok();
        let stat = spread_ns(
            "one fstat of an open handle",
            N,
            |_| (),
            |()| {
                std::hint::black_box(held.as_ref().map(fs::File::metadata));
            },
        );
        let _large = spread_ns(
            "safe_open::open_read_len, 4 MiB",
            N,
            |_| (),
            |()| {
                std::hint::black_box(safe_open::open_read_len(&large).ok());
            },
        );
        println!(
            "  the door's p50 less the bare open's: {} ns; one fstat's p50: {stat} ns",
            door.saturating_sub(plain_open)
        );
        println!("C-SF-02 the reading door is flat in the file, {label}");
        let base = super::cost_ps(200, || safe_open::open_read_len(&small).ok());
        let at = super::cost_ps(200, || safe_open::open_read_len(&large).ok());
        ok &= super::ratio("open_read_len 0 B -> 4 MiB", base, at);
    }
    println!("C-SF-02 the reading door is flat in its directory");
    let base = super::cost_ps(200, || {
        safe_open::open_read_len(&quiet.join("zz-small.bin")).ok()
    });
    let at = super::cost_ps(200, || {
        safe_open::open_read_len(&crowded.join("zz-small.bin")).ok()
    });
    ok &= super::ratio("open_read_len alone -> beside 2,000 names", base, at);
    let _ignored = fs::remove_dir_all(&quiet);
    let _ignored = fs::remove_dir_all(&crowded);
    ok
}

/// C-SF-03.
fn staging_creates() -> bool {
    let quiet = bench_dir("stage-quiet");
    let crowded = bench_dir("stage-crowded");
    crowd(&crowded);
    let mut ok = true;
    let mut by_directory = [0u128; 2];
    for (within, label) in [(&quiet, "alone"), (&crowded, "beside 2,000 names")] {
        println!("C-SF-03 staging::create, {label}: {}", within.display());
        let name = within.join("zz-staged.tmp");
        let remove = |path: &Path| {
            if fs::remove_file(path).is_err() {
                super::refuse("a staged file would not remove");
            }
        };
        let _free = spread_ns(
            "create, a free name",
            N,
            |_| (),
            |()| match staging::create(&name) {
                Ok(made) if made.cleared.is_none() => {
                    drop(made);
                    remove(&name);
                }
                _ => super::refuse("a free name was not created"),
            },
        );
        let mut leftover_p50 = [0u128; 2];
        for (slot, (bytes, n)) in [(0usize, N), (LARGE, N_LARGE)].into_iter().enumerate() {
            let body = vec![0x5A; bytes];
            let held = u64::try_from(bytes).unwrap_or(u64::MAX);
            let p50 = spread_ns(
                &format!("create over a leftover of {bytes} B"),
                n,
                |_| {
                    if fs::write(&name, &body).is_err() {
                        super::refuse("a leftover would not write");
                    }
                },
                |()| match staging::create(&name) {
                    Ok(made) if made.cleared == Some(held) => drop(made),
                    _ => super::refuse("a leftover was not cleared"),
                },
            );
            if let Some(at) = leftover_p50.get_mut(slot) {
                *at = p50;
            }
            remove(&name);
        }
        let [empty, large] = leftover_p50;
        // THE HOST'S UNLINK, PRINTED AND NOT HELD (L-R316-06).
        println!(
            "  leftover 0 B -> 4 MiB p50, {label}: {empty} ns -> {large} ns, the host's \
             unlink freeing the leftover's blocks; printed, not held (L-R316-06)"
        );
        if let Some(at) = by_directory.get_mut(usize::from(label != "alone")) {
            *at = empty;
        }
    }
    let [alone, beside] = by_directory;
    ok &= super::ratio(
        "create over a 0 B leftover alone -> beside 2,000 names",
        alone * 1_000,
        beside * 1_000,
    );
    let _ignored = fs::remove_dir_all(&quiet);
    let _ignored = fs::remove_dir_all(&crowded);
    ok
}

/// C-SF-04.
fn store_roots() {
    let dir = bench_dir("roots");
    let own = dir.join("own");
    let linked = dir.join("linked");
    let made = fs::create_dir_all(own.join(safe_open::BARS_DIR))
        .and_then(|()| fs::create_dir_all(&linked))
        .and_then(|()| {
            std::os::unix::fs::symlink(
                own.join(safe_open::BARS_DIR),
                linked.join(safe_open::BARS_DIR),
            )
        });
    if made.is_err() {
        super::refuse("the store-root bench would not build its roots");
    }
    println!("C-SF-04 refuse_linked_store_root: {}", dir.display());
    let _own = spread_ns(
        "a store with its own bars (accepted)",
        N,
        |_| (),
        |()| {
            if safe_open::refuse_linked_store_root(&own).is_err() {
                super::refuse("a store with its own bars was refused");
            }
        },
    );
    let _linked = spread_ns(
        "a store whose bars is a link (refused)",
        N,
        |_| (),
        |()| {
            if safe_open::refuse_linked_store_root(&linked).is_ok() {
                super::refuse("a linked store was accepted");
            }
        },
    );
    let _ignored = fs::remove_dir_all(&dir);
}

/// Every measurement of this file; whether each gated ratio held.
pub(crate) fn run() -> bool {
    let read = reading();
    let staged = staging_creates();
    store_roots();
    read && staged
}
