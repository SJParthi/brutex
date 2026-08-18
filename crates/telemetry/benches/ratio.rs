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

/// Every sample from one run, sorted, so a percentile can be read off it.
///
/// **WHY THIS EXISTS BESIDE [`cost_ps`].** `cost_ps` is `min` over [`TRIALS`] of
/// a MEAN, taken on sinks built with [`NO_ROTATION`]. Both halves of that hide
/// the thing `CLAUDE.md` §3 rule 4 is about: `min` discards the trial containing
/// a roll, and `NO_ROTATION` guarantees there is no roll to discard. So the
/// existing rows prove FLATNESS across file size — a real and useful property —
/// and are not evidence of a worst-case bound.
///
/// A mean cannot express a worst case. This keeps every sample, and
/// `telemetry::bench::the_tail_is_flat_in_the_size_of_the_file_too` (C-T-01b)
/// is the row that
/// uses it to gate the flatness claim at p99 under the same ceiling.
struct Dist {
    ns: Vec<u128>,
}

impl Dist {
    /// Times `op` once per rep, retaining every sample.
    fn of<T>(reps: u32, mut op: impl FnMut() -> T) -> Self {
        let mut ns = Vec::with_capacity(reps as usize);
        for _ in 0..reps {
            let start = Instant::now();
            black_box(op());
            ns.push(start.elapsed().as_nanos());
        }
        ns.sort_unstable();
        Self { ns }
    }

    /// The sample at `q` thousandths, or zero for an empty run.
    fn at(&self, permille: usize) -> u128 {
        if self.ns.is_empty() {
            return 0;
        }
        let last = self.ns.len() - 1;
        let idx = (self.ns.len() * permille / 1_000).min(last);
        self.ns.get(idx).copied().unwrap_or(0)
    }

    fn max(&self) -> u128 {
        self.ns.last().copied().unwrap_or(0)
    }

    /// Prints the shape. Never fails a gate — it REPORTS, because there is no
    /// agreed ceiling for these yet and inventing one would be the same mistake
    /// in the other direction.
    fn report(&self, label: &str) {
        println!(
            "  {label:<52} p50 {:>10} ns  p99 {:>10} ns  max {:>10} ns  n={}",
            self.at(500),
            self.at(990),
            self.max(),
            self.ns.len()
        );
    }
}

/// **THE FLATNESS CLAIM, AT p99 INSTEAD OF AT THE MEAN.**
///
/// C-T-01 asserts that one `emit` costs the same with 1,000, 10,000 and 100,000
/// events already in the file. It proves that with `min` over twenty trials of a
/// MEAN, and a mean cannot see a tail: an emit that occasionally took a thousand
/// times longer would move it by a fraction of a percent and the row would stay
/// green.
///
/// **This gate invents no new threshold, which is the point.** It applies the
/// SAME [`CEILING_PERMILLE`] to the SAME claim, measured with a statistic that
/// can actually fail — the 99th percentile of the per-call distribution. If the
/// tail starts growing with the size of the file, the flatness claim is false
/// and this is the row that says so.
///
/// Rotation stays out of it, and stays a report rather than a gate
/// (`the_worst_case_is_named_rather_than_averaged_away`), because there is no
/// agreed ceiling for what a roll may cost and inventing one here would be the
/// same mistake in the other direction: a number with no evidence behind it,
/// tuned later until it passes.
fn the_tail_is_flat_in_the_size_of_the_file_too() -> bool {
    let mut ok = true;
    let at = |preload: u32, name: &str| -> u128 {
        let (sink, dir) = loaded(name, preload);
        let d = Dist::of(4_000, || {
            sink.emit(&Event::info("bench", "one timed event"))
        });
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
        d.at(990)
    };
    let small = at(SMALL, "p99-small");
    let medium = at(MEDIUM, "p99-medium");
    let large = at(LARGE, "p99-large");

    ok &= ratio(
        "C-T-01b emit p99: 1,000 -> 10,000 events in file",
        small,
        medium,
    );
    ok &= ratio(
        "C-T-01b emit p99: 1,000 -> 100,000 events in file",
        small,
        large,
    );
    ok
}

/// **THE WORST CASE, WITH ROTATION INSIDE THE TIMED REGION.**
///
/// The three rows above answer "is the cost flat in the size of the file". This
/// answers the different question `CLAUDE.md` §3 rule 4 actually asks: what does
/// the most expensive single call cost, at the SHIPPED configuration, where
/// rolls happen.
///
/// It is a report and not a gate. The design is O(1) — a roll is at most
/// `2 * keep_files` syscalls with `keep_files` a compile-time `u8`, and no term
/// is a function of events already logged — but the CONSTANT was never
/// measured, and every document describing this crate quotes a ratio of means as
/// though it were a bound. Naming the number is the fix; picking a ceiling for
/// it is a separate decision with no evidence behind it yet. The numbers it
/// prints are recorded in `docs/06-limits.md` §46; the flatness claim beside
/// them IS gated, by
/// `telemetry::bench::the_tail_is_flat_in_the_size_of_the_file_too` (C-T-01b).
fn the_worst_case_is_named_rather_than_averaged_away() {
    println!("worst case — shipped config, rotation INSIDE the timed region");

    // ORDINARY: the shipped bound, but too few events to roll.
    let dir = scratch("dist-ordinary");
    let Ok(sink) = Sink::open(&Config::new(dir.clone())) else {
        refuse("a sink over a scratch directory")
    };
    let ordinary = Dist::of(20_000, || sink.emit(&Event::info("bench", "ordinary")));
    ordinary.report("emit, shipped config, no roll reached");
    drop(sink);
    let _ignored = std::fs::remove_dir_all(&dir);

    // ROLLING: a bound small enough that a roll lands inside the samples.
    let dir = scratch("dist-rolling");
    let Ok(sink) = Sink::open(
        &Config::new(dir.clone())
            .with_max_file_bytes(telemetry::MIN_FILE_BYTES)
            .with_keep_files(8),
    ) else {
        refuse("a rolling sink over a scratch directory")
    };
    let rolling = Dist::of(20_000, || sink.emit(&Event::info("bench", "rolling")));
    rolling.report("emit, 1 KiB x 8 — every few events ROLLS");
    println!(
        "  {:<52} {}x the median ordinary emit",
        "the rolling max, against the ordinary p50",
        if ordinary.at(500) == 0 {
            0
        } else {
            rolling.max() / ordinary.at(500)
        }
    );
    let health = sink.health();
    println!(
        "  {:<52} {} rotation(s), {} dropped",
        "and it really did roll", health.rotations, health.dropped
    );
    drop(sink);
    let _ignored = std::fs::remove_dir_all(&dir);

    // CONTENDED: crates/api is a multi-threaded runtime, and every number in
    // every document describing this sink is single-threaded.
    for threads in [1_usize, 4, 8] {
        let dir = scratch(&format!("dist-threads-{threads}"));
        let Ok(opened) = Sink::open(&Config::new(dir.clone())) else {
            refuse("a sink over a scratch directory")
        };
        let sink = std::sync::Arc::new(opened);
        let per = 4_000_u32;
        let worst = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..threads)
                .map(|_| {
                    let sink = std::sync::Arc::clone(&sink);
                    scope.spawn(move || {
                        Dist::of(per, || sink.emit(&Event::info("bench", "contended")))
                    })
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).fold(
                Dist { ns: Vec::new() },
                |mut all, d| {
                    all.ns.extend(d.ns);
                    all.ns.sort_unstable();
                    all
                },
            )
        });
        worst.report(&format!("emit, {threads} thread(s), shipped config"));
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
    }
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

/// The per-emit floor: a filtered event on the same sink.
///
/// # Why a ratio alone cannot see a regression
///
/// Every row in this file divides one emit cost by another emit cost, and a
/// UNIFORM slowdown cancels in a quotient. An audit measured exactly that
/// elsewhere in this workspace: a mask operation **174x slower passed its
/// crate's ratio rows at 0.98x-1.00x**, because both legs moved together.
/// `vocab`, `indicators`, `engine` and now `core` each carry a floor-relative
/// budget for that reason; this crate did not.
///
/// The denominator has to be something that cannot move when the WRITE path
/// does. A filtered event is that: it enters `Sink::emit` and returns at the
/// first level comparison, having touched no clock, no lock and no buffer. Same
/// call, same sink, same dispatch — everything except the work being measured.
/// So the quotient is "how many level checks does one written event cost".
fn floor_ps(sink: &Sink) -> u128 {
    cost_ps(2_000, || {
        // Below the sink's default minimum, so it returns at the first compare.
        let event = Event::trace("bench", "the floor");
        black_box(sink).emit(black_box(&event))
    })
}

/// Prints one budget in floors and returns whether it held.
///
/// A breached RATIO says the cost depends on how full the file is. A breached
/// BUDGET says the cost rose for every file size at once, which no ratio in this
/// file can report.
fn budget(label: &str, floor: u128, at_ps: u128, allowed: u128) -> bool {
    if floor == 0 {
        println!("  {label:<58} UNMEASURABLE — the floor timed at zero");
        return false;
    }
    let floors = at_ps.saturating_mul(1_000) / floor;
    let ok = floors <= allowed.saturating_mul(1_000);
    println!(
        "  {label:<58} {at_ps:>8} ps = {}.{:03} floors, budget {allowed}   {}",
        floors / 1_000,
        floors % 1_000,
        if ok { "ok" } else { "OVER BUDGET" }
    );
    ok
}

/// C-T-04 — one written emit costs a bounded multiple of the per-emit floor.
fn emit_stays_within_its_budget() -> bool {
    /// Floors allowed per written event.
    ///
    /// Measured, arm64 laptop, release, four consecutive runs: **173.5, 298.2,
    /// 219.3, 329.5** floors, at a floor of 5,895–6,937 ps.
    ///
    /// # The spread is 1.9x, and that is honest rather than hidden
    ///
    /// `crates/core`'s equivalent budget varies by 1.06x between runs because
    /// both its legs are pointer-chasing. This one does not: the numerator is a
    /// written event, which reaches the filesystem, and the denominator is a
    /// level comparison that does not. The variance is the disk's, and no
    /// arrangement of this bench removes it.
    ///
    /// **So this row cannot resolve a regression under about 3x**, and it is not
    /// claimed to. What it CAN do is what no ratio in this file can: catch a
    /// slowdown that moves every file size at once. The 174x uniform regression
    /// measured elsewhere in this workspace — which passed its crate's ratio rows
    /// at 0.98x — would read about 57,000 floors here and be refused by a factor
    /// of 57.
    ///
    /// **1,000**, sized on the worst observed run with roughly 3x left over,
    /// which is the same rule `engine` and `core` apply and a wider margin
    /// because the measurement is noisier.
    const ALLOWED: u128 = 1_000;

    let (sink, _d) = loaded("emit-budget", MEDIUM);
    let floor = floor_ps(&sink);
    println!("  the per-emit floor is {floor} ps — one filtered event, same sink");
    let written = cost_ps(200, || {
        let event = Event::info("bench", "one timed event").with("k", 1_i64);
        black_box(&sink).emit(black_box(&event))
    });
    budget(
        "C-T-04 written emit against the floor",
        floor,
        written,
        ALLOWED,
    )
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
    ok &= emit_stays_within_its_budget();
    ok &= a_filtered_event_touches_nothing_and_stays_flat();
    ok &= the_tail_is_flat_in_the_size_of_the_file();
    ok &= the_tail_is_flat_in_the_size_of_the_file_too();
    println!();
    the_worst_case_is_named_rather_than_averaged_away();
    println!();
    if ok {
        println!("all ratios within the ceiling");
    } else {
        println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
        std::process::exit(1);
    }
}
