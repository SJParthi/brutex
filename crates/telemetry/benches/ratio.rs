//! Gate 8 for `crates/telemetry` — one `emit` costs the same whatever is
//! already in the file, and the tail costs the same whatever the file weighs.
//!
//! # What is measured, and what is deliberately not
//!
//! This crate makes two cost claims and they are about two different sizes.
//!
//! [`telemetry::sink`] says one `emit` touches nothing that grows: a level
//! check, a clock read, an uncontended lock, one pass over the event's own
//! bytes into a buffer the sink reuses, one integer comparison against a
//! running byte count, one `write`. Nothing in that list is a function of how
//! many events came before or how large the file is — so the measurement is
//! per-emit cost against **events already written**.
//!
//! [`telemetry::tail`] says reading the last N events is O(N × line width) and
//! **O(1) in the size of the file**, because the file is walked backwards from
//! its end in [`READ_BLOCK`]-byte blocks and stops when it has what was asked
//! for. So the measurement there is the cost of the same twenty events against
//! a file that is a hundred times bigger.
//!
//! What is NOT measured, because measuring it would measure the kernel: the
//! `write` syscall's own latency, and the `metadata`/`open`/`seek` the tail
//! makes per file touched. `sink.rs`'s header already says a syscall is not
//! free and does not claim otherwise; this bench measures the part this crate
//! controls, which is that neither cost grows.
//!
//! Rotation is kept out of the way on purpose. Every sink here is opened with
//! a file ceiling far above what the bench writes, because a rotation is a
//! constant that happens once per `max_file_bytes` and folding it into a
//! per-emit average would measure how often it was triggered rather than what
//! an emit costs. `sink.rs` names that cost separately and it is not this
//! file's subject.
//!
//! See `crates/store/benches/ratio.rs` for why there is no benchmarking
//! framework and why every ratio here is integer arithmetic.

use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Instant;

use telemetry::{Config, Event, Query, Sink, tail};

/// A ratio above this is a failure. Thousandths, so the comparison is integer.
///
/// 3.0x — `docs/04-invariants.md`, the shared-CI number.
const CEILING_PERMILLE: u128 = 3_000;

/// How many times each measurement is repeated before the minimum is taken.
const TRIALS: u32 = 20;

/// Events already in the file when an emit is timed, and when a tail is.
const SMALL: u32 = 1_000;
const MEDIUM: u32 = 10_000;
const LARGE: u32 = 100_000;

/// Far above anything this bench writes, so no rotation happens inside a
/// timed region. See the module header.
const NO_ROTATION: u64 = 1 << 40;

/// How many events the tail is asked for. Twenty is what the audit console
/// asks for, and the claim is that this number — not the file's size — is
/// what the cost tracks.
const TAIL_LIMIT: usize = 20;

/// Times one closure, in picoseconds per call, as a minimum over [`TRIALS`].
fn cost_ps<T>(reps: u32, mut op: impl FnMut() -> T) -> u128 {
    let mut best = u128::MAX;
    for _ in 0..TRIALS {
        let start = Instant::now();
        for _ in 0..reps {
            black_box(op());
        }
        let ps = start.elapsed().as_nanos() * 1_000 / u128::from(reps);
        if ps < best {
            best = ps;
        }
    }
    best
}

/// Prints one measurement and returns whether it stayed under the ceiling.
fn ratio(label: &str, base_ps: u128, at_ps: u128) -> bool {
    if base_ps == 0 {
        println!("  {label:<52} UNMEASURABLE — the 1x baseline timed at zero");
        return false;
    }
    let permille = at_ps * 1_000 / base_ps;
    let ok = permille <= CEILING_PERMILLE;
    println!(
        "  {label:<52} {:>9} ps -> {:>9} ps   ratio {}.{:03}x  {}",
        base_ps,
        at_ps,
        permille / 1_000,
        permille % 1_000,
        if ok { "ok" } else { "BREACH" }
    );
    ok
}

/// A scratch directory of this process's own, emptied first.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-telemetry-bench-{}-{name}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&dir);
    dir
}

/// Refuses loudly rather than unwrapping into a panic message nobody reads.
fn refuse(what: &str) -> ! {
    println!("BENCH SETUP FAILED — {what}");
    std::process::exit(1)
}

/// A sink over a fresh directory, with `preload` events already written.
fn loaded(name: &str, preload: u32) -> (Sink, PathBuf) {
    let dir = scratch(name);
    let config = Config::new(dir.clone()).with_max_file_bytes(NO_ROTATION);
    let Ok(sink) = Sink::open(&config) else {
        refuse("a sink over a scratch directory")
    };
    for i in 0..preload {
        let event = Event::info("bench", "one preloaded event").with("i", i64::from(i));
        if !sink.emit(&event).is_written() {
            refuse("a preloaded event was not written");
        }
    }
    (sink, dir)
}

/// The claim: one emit does not get more expensive as the file fills.
fn emit_cost_does_not_grow_with_the_file() -> bool {
    let (small, _d1) = loaded("emit-small", SMALL);
    let (medium, _d2) = loaded("emit-medium", MEDIUM);
    let (large, _d3) = loaded("emit-large", LARGE);

    // The same event every time, so the only thing that differs between the
    // three measurements is how much is already in the file.
    let one = |sink: &Sink| {
        cost_ps(200, || {
            let event = Event::info("bench", "one timed event").with("k", 1_i64);
            black_box(sink).emit(black_box(&event))
        })
    };

    let base = one(&small);
    let mut ok = true;
    ok &= ratio(
        "C-T-01 emit: 1,000 -> 10,000 events in file",
        base,
        one(&medium),
    );
    ok &= ratio(
        "C-T-01 emit: 1,000 -> 100,000 events in file",
        base,
        one(&large),
    );
    ok
}

/// A filtered event returns at the level check having touched nothing else —
/// no clock, no lock, no buffer — so it too cannot grow with the file.
fn a_filtered_event_touches_nothing_and_stays_flat() -> bool {
    let (small, _d1) = loaded("filtered-small", SMALL);
    let (large, _d2) = loaded("filtered-large", LARGE);

    // Below the sink's default minimum, so every one of these returns at the
    // first comparison.
    let filtered = |sink: &Sink| {
        cost_ps(2_000, || {
            let event = Event::trace("bench", "below the floor");
            black_box(sink).emit(black_box(&event))
        })
    };

    let base = filtered(&small);
    let mut ok = true;
    ok &= ratio(
        "C-T-02 filtered emit: 1,000 -> 100,000 events",
        base,
        filtered(&large),
    );
    // And the whole point of the early return: a filtered event is cheaper
    // than a written one on the SAME sink. Measured, not asserted.
    let written = cost_ps(200, || {
        let event = Event::info("bench", "one timed event").with("k", 1_i64);
        black_box(&small).emit(black_box(&event))
    });
    ok &= ratio("C-T-02 filtered against written, same sink", written, base);
    ok
}

/// The tail reads the last twenty events without reading the file.
fn the_tail_is_flat_in_the_size_of_the_file() -> bool {
    let (small, ds) = loaded("tail-small", SMALL);
    let (medium, dm) = loaded("tail-medium", MEDIUM);
    let (large, dl) = loaded("tail-large", LARGE);
    // Drop the sinks so every byte is on disk before anything is read back.
    drop(small);
    drop(medium);
    drop(large);

    let last = |dir: &Path| {
        cost_ps(20, || {
            let query = Query::last(TAIL_LIMIT);
            tail(black_box(dir), 8, black_box(&query))
        })
    };

    let base = last(&ds);
    let mut ok = true;
    ok &= ratio("C-T-03 tail(20): 1,000 -> 10,000 events", base, last(&dm));
    ok &= ratio("C-T-03 tail(20): 1,000 -> 100,000 events", base, last(&dl));

    // The bytes actually read are the sharper statement: if the walk were a
    // function of the file, this number would grow with it.
    let query = Query::last(TAIL_LIMIT);
    let read_small = tail(&ds, 8, &query).bytes_read;
    let read_large = tail(&dl, 8, &query).bytes_read;
    println!(
        "  {:<52} {read_small} bytes -> {read_large} bytes",
        "C-T-03 bytes read for the same 20 events"
    );
    if read_large > read_small.saturating_mul(2) {
        println!("  C-T-03 BREACH — the tail read more bytes on the larger file");
        ok = false;
    }
    ok
}

fn main() {
    println!("gate 8 — crates/telemetry, ceiling {CEILING_PERMILLE} permille");
    let mut ok = true;
    ok &= emit_cost_does_not_grow_with_the_file();
    ok &= a_filtered_event_touches_nothing_and_stays_flat();
    ok &= the_tail_is_flat_in_the_size_of_the_file();
    if ok {
        println!("all ratios within the ceiling");
    } else {
        println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
        std::process::exit(1);
    }
}
