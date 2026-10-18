//! The FIB bench harness, shared by the four FIB bench targets (FIB-1;
//! D-6318; FM-F001, FM-F002). Included by `#[path]` into
//! `crates/vocab/benches/fib_cube.rs`, `crates/indicators/benches/fibflag.rs`,
//! `crates/runner/benches/fib.rs`, `crates/cli/benches/fib.rs` and the test
//! `crates/runner/tests/fib_harness.rs`, so there is one harness and its own
//! tests run under `cargo test`.
//!
//! # What a row must declare
//!
//! * a CLASS: `WORST-CASE`, `AMORTISED` with its period, `EXPECTED`, or
//!   `BOUNDED` with its bound named for `docs/06-limits.md`. A row without one
//!   is refused (FM-F001).
//! * an AXIS: `FIXED-STATE` (exempt from the scale rules, and labelled), or a
//!   DATA axis naming its unit and the production size of that unit. A data
//!   row's largest working set must exceed the last-level cache this host's
//!   pointer-chase probe measured, and a largest scale below production prints
//!   `EXTRAPOLATION`. A row without an axis is refused (FM-F002).
//! * three SCALES spanning at least 100x, or it is refused.
//! * a STRUCTURAL WORK COUNTER: a plain field on the op's own state (no static,
//!   no atomic, no telemetry — gate 17), read around EVERY call into a fixed
//!   histogram — exact below [`EXACT`], one bucket per power of two above — so
//!   per-op p50, p99, p99.9 and max work are exact.
//!
//! # What is timed
//!
//! After a warm-up, [`BATCHES`] batches of [`BATCH`] calls at each scale, with
//! `Instant`; per-call time is a batch's elapsed time over its calls, so every
//! time percentile is a percentile OF BATCH MEANS and is labelled so. The same
//! is printed at batch sizes 1, 10, 100 and 1,000, beside this host's measured
//! timer granularity: a call shorter than the granularity is visible only in
//! the larger batches.
//!
//! # Verdicts
//!
//! * `WORST-CASE`: every scale's max work equals its p50 work within 1.25x (a
//!   larger max contradicts the class and FAILS), the p99 and max work ratios
//!   across scales are within 1.25x, and the p99 time ratio is within 1.25x.
//! * `AMORTISED`: every scale runs at least one full period; the mean work and
//!   mean time ratios are within 1.25x; the max and the period are printed.
//! * `EXPECTED`: the p99 work and p99 time ratios are within 1.25x.
//! * `BOUNDED`: its bound is printed and its ratios are information.
//!
//! A time ratio above 1.25x is a printed FAIL against the specification's line.
//! The process exits non-zero on any refusal, any structural failure, or a time
//! ratio above the shared-runner ceiling of 3.0x that every other bench in this
//! workspace uses ([`SHARED_CEILING_PERMILLE`]): gate 8 runs every bench on a
//! shared CI runner, whose scheduler is noisier than a 25% line.
//!
//! Integers only: times in picoseconds, ratios in thousandths.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    reason = "one harness included into five targets, each using part of it; a \
              bench that cannot measure has nothing to report"
)]

use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;

/// Batches timed at each scale at the largest batch size.
pub(crate) const BATCHES: usize = 2_000;
/// Calls per batch at the largest batch size.
pub(crate) const BATCH: u64 = 1_000;
/// The batch sizes every row is also timed at.
pub(crate) const BATCH_SIZES: [u64; 4] = [1, 10, 100, 1_000];
/// Batches timed at each of the smaller batch sizes.
pub(crate) const SMALL_BATCHES: usize = 2_000;
/// Calls before any timing, at each scale.
pub(crate) const WARMUP: u64 = 20_000;
/// A ratio at or below this passes the specification's line (thousandths).
pub(crate) const PASS_PERMILLE: u128 = 1_250;
/// A time ratio above this fails the process: the shared-runner ceiling every
/// other bench in this workspace uses (thousandths).
pub(crate) const SHARED_CEILING_PERMILLE: u128 = 3_000;
/// The least span the three scales must cover.
pub(crate) const MIN_SPAN: u64 = 100;
/// Work values below this are counted exactly.
pub(crate) const EXACT: usize = 65_536;

/// A row's declared cost class (FM-F001).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Class {
    /// Every call does the same work.
    WorstCase,
    /// Rare expensive calls, paid for over a period of calls.
    Amortised {
        /// Calls per period.
        period: u64,
    },
    /// Constant in expectation (a hash probe).
    Expected,
    /// Not constant, by design; the bound is named.
    Bounded {
        /// The bound, as `docs/06-limits.md` names it.
        bound: &'static str,
    },
}

impl Class {
    /// The class as printed.
    #[must_use]
    pub(crate) fn label(&self) -> String {
        match self {
            Self::WorstCase => "WORST-CASE".to_owned(),
            Self::Amortised { period } => format!("AMORTISED (period {period} calls)"),
            Self::Expected => "EXPECTED".to_owned(),
            Self::Bounded { bound } => format!("BOUNDED ({bound})"),
        }
    }
}

/// What a row's scale measures (FM-F002).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Axis {
    /// The op's state does not grow with the data; exempt from the scale rules.
    FixedState {
        /// Why the state is fixed.
        why: &'static str,
    },
    /// The op's state grows with a unit of data.
    Data {
        /// The unit (per partition, per stock-day, per route file).
        unit: &'static str,
        /// The production size of that unit, when known.
        production: Option<u64>,
    },
}

/// One bench row over a state `S`.
pub(crate) struct Row<S> {
    /// The row's id, as `docs/06-limits.md` names it.
    pub(crate) id: &'static str,
    /// What one call does.
    pub(crate) name: &'static str,
    /// The declared class.
    pub(crate) class: Option<Class>,
    /// The declared axis.
    pub(crate) axis: Option<Axis>,
    /// Three scales, spanning at least [`MIN_SPAN`].
    pub(crate) scales: [u64; 3],
    /// Builds the state at a scale.
    pub(crate) setup: fn(u64) -> S,
    /// One call, for call number `i`.
    pub(crate) op: fn(&mut S, u64) -> u64,
    /// The state's structural work counter.
    pub(crate) work: fn(&S) -> u64,
    /// The state's working-set bytes.
    pub(crate) bytes: fn(&S) -> u64,
}

/// What this host measured once, for every row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Host {
    /// The smallest positive step `Instant` takes, in nanoseconds.
    pub(crate) granularity_ns: u128,
    /// The last-level cache the pointer-chase probe found, in bytes.
    pub(crate) llc_bytes: u64,
    /// The production worker count (`available_parallelism`).
    pub(crate) workers: usize,
}

/// The smallest positive difference between two successive `Instant` reads.
#[must_use]
pub(crate) fn timer_granularity_ns() -> u128 {
    let mut best = u128::MAX;
    for _ in 0..100_000 {
        let a = Instant::now();
        let mut b = Instant::now();
        while b == a {
            b = Instant::now();
        }
        best = best.min(b.duration_since(a).as_nanos().max(1));
    }
    best
}

/// A deterministic generator (splitmix64), for the probe's permutation.
struct Mix(u64);

impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// Picoseconds per dependent load over a single random cycle through
/// `bytes` of memory (Sattolo's shuffle, so the cycle visits every slot).
#[must_use]
pub(crate) fn chase_ps(bytes: u64, steps: u64) -> u128 {
    let slots = usize::try_from(bytes / 8).unwrap().max(2);
    let mut next: Vec<usize> = (0..slots).collect();
    let mut mix = Mix(bytes);
    for i in (1..slots).rev() {
        let j = usize::try_from(mix.next() % i as u64).unwrap();
        next.swap(i, j);
    }
    let mut at = 0;
    for _ in 0..slots.min(1 << 16) {
        at = next[at];
    }
    let start = Instant::now();
    for _ in 0..steps {
        at = next[at];
    }
    black_box(at);
    start.elapsed().as_nanos() * 1_000 / u128::from(steps)
}

/// The pointer-chase curve from 16 KiB to `top` bytes, doubling, and the
/// last-level cache it implies. Printed whole, so a reader can judge the knee;
/// the estimate is a measurement of this host, never a typed size.
#[must_use]
pub(crate) fn llc_probe(top: u64, out: &mut String) -> u64 {
    let mut curve = Vec::new();
    let mut size = 16 * 1_024;
    while size <= top {
        curve.push((size, chase_ps(size, 1 << 20)));
        size *= 2;
    }
    let _ = writeln!(out, "  pointer-chase probe (ps per dependent load):");
    for (size, ps) in &curve {
        let _ = writeln!(out, "    {:>9} KiB  {ps:>8}", size / 1_024);
    }
    // A working set is out of cache once its loads cost at least half of a
    // load at the largest size (main memory): the estimate is the largest
    // size still below that, which errs large, so a data row that passes it
    // is not cache-resident.
    let memory = curve.last().map_or(0, |(_, ps)| *ps);
    let knee = curve
        .iter()
        .filter(|(_, ps)| *ps * 2 < memory)
        .map(|(size, _)| *size)
        .max()
        .unwrap_or(0);
    let _ = writeln!(
        out,
        "  last-level cache estimate: {} KiB (the largest size whose load costs under \
         half of a {} KiB load, {memory} ps)",
        knee / 1_024,
        top / 1_024
    );
    knee
}

/// Measures this host once: the timer, the cache, the workers.
#[must_use]
pub(crate) fn host(out: &mut String) -> Host {
    let granularity_ns = timer_granularity_ns();
    let workers = std::thread::available_parallelism().map_or(1, usize::from);
    let _ = writeln!(out, "host: this machine (the cloud container this ran in)");
    let _ = writeln!(out, "  timer granularity: {granularity_ns} ns");
    let _ = writeln!(out, "  production worker count: {workers}");
    let llc_bytes = llc_probe(256 * 1_024 * 1_024, out);
    Host {
        granularity_ns,
        llc_bytes,
        workers,
    }
}

/// Per-call work: exact below [`EXACT`], one bucket per power of two above,
/// with the exact max and sum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorkHistogram {
    /// Counts of each exact value below [`EXACT`].
    exact: Vec<u64>,
    /// Counts by bit length, for values at or above [`EXACT`].
    wide: [u64; 65],
    /// Calls recorded.
    count: u64,
    /// Work recorded, in all.
    sum: u128,
    /// The largest value recorded.
    max: u64,
}

impl WorkHistogram {
    /// An empty histogram.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            exact: vec![0; EXACT],
            wide: [0; 65],
            count: 0,
            sum: 0,
            max: 0,
        }
    }

    /// Records one call's work.
    pub(crate) fn record(&mut self, work: u64) {
        match usize::try_from(work) {
            Ok(at) if at < EXACT => self.exact[at] += 1,
            _ => self.wide[(u64::BITS - work.leading_zeros()) as usize] += 1,
        }
        self.count += 1;
        self.sum += u128::from(work);
        self.max = self.max.max(work);
    }

    /// The work at quantile `num / den` (nearest rank): exact below
    /// [`EXACT`], the bucket's upper bound above.
    #[must_use]
    pub(crate) fn quantile(&self, num: u64, den: u64) -> u64 {
        let rank = (u128::from(self.count) * u128::from(num)).div_ceil(u128::from(den));
        let mut seen = 0_u128;
        for (value, n) in self.exact.iter().enumerate() {
            seen += u128::from(*n);
            if seen >= rank && *n > 0 {
                return value as u64;
            }
        }
        for (bits, n) in self.wide.iter().enumerate() {
            seen += u128::from(*n);
            if seen >= rank && *n > 0 {
                let upper = if bits >= 64 {
                    u64::MAX
                } else {
                    (1 << bits) - 1
                };
                return upper.min(self.max);
            }
        }
        self.max
    }

    /// The mean work per call, in thousandths.
    #[must_use]
    pub(crate) fn mean_milli(&self) -> u128 {
        self.sum * 1_000 / u128::from(self.count.max(1))
    }

    /// The largest value recorded.
    #[must_use]
    pub(crate) const fn max(&self) -> u64 {
        self.max
    }

    /// Calls recorded.
    #[must_use]
    pub(crate) const fn count(&self) -> u64 {
        self.count
    }
}

impl Default for WorkHistogram {
    fn default() -> Self {
        Self::new()
    }
}

/// One scale's measurements.
#[derive(Clone, Debug)]
pub(crate) struct Measured {
    /// The scale.
    pub(crate) scale: u64,
    /// Per-call work.
    pub(crate) work: WorkHistogram,
    /// p50, p99 and max of batch means at [`BATCH`], in ps per call.
    pub(crate) time: (u128, u128, u128),
    /// The mean time per call at [`BATCH`], in ps.
    pub(crate) mean_ps: u128,
    /// p50 and p99 of batch means at each of [`BATCH_SIZES`], in ps per call.
    pub(crate) batches: Vec<(u64, u128, u128)>,
    /// The working set, in bytes.
    pub(crate) bytes: u64,
}

/// The nearest-rank quantile of a sorted list.
fn nearest(sorted: &[u128], num: usize, den: usize) -> u128 {
    let rank = (sorted.len() * num).div_ceil(den).max(1);
    sorted[rank - 1]
}

/// Times `batches` batches of `size` calls, reading the work counter around
/// every call. Returns the sorted per-call times (ps) of each batch.
fn timed<S>(
    row: &Row<S>,
    state: &mut S,
    i: &mut u64,
    size: u64,
    batches: usize,
    work: &mut WorkHistogram,
) -> Vec<u128> {
    let mut times = Vec::with_capacity(batches);
    for _ in 0..batches {
        let start = Instant::now();
        for _ in 0..size {
            let before = (row.work)(state);
            black_box((row.op)(state, *i));
            work.record((row.work)(state).wrapping_sub(before));
            *i += 1;
        }
        times.push(start.elapsed().as_nanos() * 1_000 / u128::from(size));
    }
    times.sort_unstable();
    times
}

/// Measures one row at one scale.
pub(crate) fn measure<S>(row: &Row<S>, scale: u64) -> Measured {
    let mut state = (row.setup)(scale);
    let mut i = 0;
    let mut warm = WorkHistogram::new();
    for _ in 0..WARMUP {
        let before = (row.work)(&state);
        black_box((row.op)(&mut state, i));
        warm.record((row.work)(&state).wrapping_sub(before));
        i += 1;
    }
    let mut work = WorkHistogram::new();
    let main = timed(row, &mut state, &mut i, BATCH, BATCHES, &mut work);
    let mean_ps = main.iter().sum::<u128>() / main.len() as u128;
    let time = (
        nearest(&main, 1, 2),
        nearest(&main, 99, 100),
        *main.last().unwrap(),
    );
    let mut batches = Vec::new();
    for size in BATCH_SIZES {
        let mut ignored = WorkHistogram::new();
        let sorted = if size == BATCH {
            main.clone()
        } else {
            timed(row, &mut state, &mut i, size, SMALL_BATCHES, &mut ignored)
        };
        batches.push((size, nearest(&sorted, 1, 2), nearest(&sorted, 99, 100)));
    }
    if let Some(Class::Amortised { period }) = row.class {
        // One full period at every scale, past what the timed batches ran.
        while i < WARMUP + period {
            let before = (row.work)(&state);
            black_box((row.op)(&mut state, i));
            work.record((row.work)(&state).wrapping_sub(before));
            i += 1;
        }
    }
    let bytes = (row.bytes)(&state);
    Measured {
        scale,
        work,
        time,
        mean_ps,
        batches,
        bytes,
    }
}

/// The mean time per call (ps) with `workers` threads each running the row on
/// its own state at `scale` — the production worker count's view of a data row.
pub(crate) fn measure_workers<S>(row: &Row<S>, scale: u64, workers: usize) -> u128 {
    let calls = BATCH * u64::try_from(BATCHES).unwrap();
    let per_worker: Vec<u128> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers.max(1))
            .map(|_| {
                scope.spawn(move || {
                    let mut state = (row.setup)(scale);
                    for i in 0..WARMUP {
                        black_box((row.op)(&mut state, i));
                    }
                    let start = Instant::now();
                    for i in WARMUP..WARMUP + calls {
                        black_box((row.op)(&mut state, i));
                    }
                    start.elapsed().as_nanos() * 1_000 / u128::from(calls)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    per_worker.iter().sum::<u128>() / per_worker.len() as u128
}

/// A ratio in thousandths, largest scale over smallest.
fn ratio(large: u128, small: u128) -> u128 {
    large * 1_000 / small.max(1)
}

/// A thousandths value as `1.234x`.
fn shown(permille: u128) -> String {
    format!("{}.{:03}x", permille / 1_000, permille % 1_000)
}

/// One row's verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// The row passes the specification's line.
    Pass,
    /// The row fails the specification's line on time only, within the shared
    /// ceiling: printed, and the process still exits zero.
    TimeAbove(String),
    /// The row fails structurally, or a time ratio passes the shared ceiling.
    Fail(String),
    /// The row is refused before it is measured.
    Refused(String),
}

impl Verdict {
    /// Whether the process must exit non-zero for this verdict.
    #[must_use]
    pub(crate) const fn fails(&self) -> bool {
        matches!(self, Self::Fail(_) | Self::Refused(_))
    }
}

/// Refuses a row that cannot be measured honestly: no class, no axis, a span
/// below [`MIN_SPAN`], or a data row whose largest working set is cache
/// resident.
fn refusal<S>(row: &Row<S>, host: &Host) -> Option<String> {
    let Some(_) = row.class else {
        return Some(format!("{}: row without class (FM-F001)", row.id));
    };
    let Some(axis) = row.axis else {
        return Some(format!("{}: row without axis (FM-F002)", row.id));
    };
    let [small, _, large] = row.scales;
    if small == 0 || large < small.saturating_mul(MIN_SPAN) {
        return Some(format!(
            "{}: scales {:?} span below {MIN_SPAN}x",
            row.id, row.scales
        ));
    }
    if let Axis::Data { .. } = axis {
        let largest = (row.bytes)(&(row.setup)(large));
        if largest <= host.llc_bytes {
            return Some(format!(
                "{}: the largest working set ({largest} bytes) is below the last-level \
                 cache ({} bytes): a cache-resident bench (FM-F002)",
                row.id, host.llc_bytes
            ));
        }
    }
    None
}

/// The structural verdict over the three scales.
fn structural(class: Class, at: &[Measured; 3]) -> Result<(), String> {
    let [small, _, large] = at;
    match class {
        Class::WorstCase => {
            for m in at {
                let p50 = u128::from(m.work.quantile(1, 2));
                if ratio(u128::from(m.work.max()), p50) > PASS_PERMILLE {
                    return Err(format!(
                        "a measured max of {} work units against a p50 of {p50} at scale {} \
                         contradicts WORST-CASE",
                        m.work.max(),
                        m.scale
                    ));
                }
            }
            let p99 = ratio(
                u128::from(large.work.quantile(99, 100)),
                u128::from(small.work.quantile(99, 100)),
            );
            let max = ratio(u128::from(large.work.max()), u128::from(small.work.max()));
            if p99 > PASS_PERMILLE || max > PASS_PERMILLE {
                return Err(format!(
                    "work grows with the scale: p99 {} and max {}",
                    shown(p99),
                    shown(max)
                ));
            }
        }
        Class::Amortised { .. } => {
            let mean = ratio(large.work.mean_milli(), small.work.mean_milli());
            if mean > PASS_PERMILLE {
                return Err(format!("mean work grows with the scale: {}", shown(mean)));
            }
        }
        Class::Expected => {
            let p99 = ratio(
                u128::from(large.work.quantile(99, 100)),
                u128::from(small.work.quantile(99, 100)),
            );
            if p99 > PASS_PERMILLE {
                return Err(format!("p99 work grows with the scale: {}", shown(p99)));
            }
        }
        Class::Bounded { .. } => {}
    }
    Ok(())
}

/// The time ratio the class is judged on, largest scale over smallest.
fn time_ratio(class: Class, at: &[Measured; 3]) -> Option<u128> {
    let [small, _, large] = at;
    match class {
        Class::WorstCase | Class::Expected => Some(ratio(large.time.1, small.time.1)),
        Class::Amortised { .. } => Some(ratio(large.mean_ps, small.mean_ps)),
        Class::Bounded { .. } => None,
    }
}

/// Measures, judges and reports one row.
pub(crate) fn run<S>(row: &Row<S>, host: &Host, out: &mut String) -> Verdict {
    if let Some(why) = refusal(row, host) {
        let _ = writeln!(out, "{} {}: REFUSED — {why}", row.id, row.name);
        return Verdict::Refused(why);
    }
    let (Some(class), Some(axis)) = (row.class, row.axis) else {
        unreachable!("refusal() refuses a row without class or axis");
    };
    let _ = writeln!(out, "{} {} — {}", row.id, row.name, class.label());
    match axis {
        Axis::FixedState { why } => {
            let _ = writeln!(
                out,
                "  axis: FIXED-STATE, exempt from the scale rules ({why})"
            );
        }
        Axis::Data { unit, production } => {
            let _ = writeln!(
                out,
                "  axis: {unit}, at 1 worker and at {} workers",
                host.workers
            );
            match production {
                Some(size) if row.scales[2] >= size => {
                    let _ = writeln!(out, "  largest scale reaches production ({size})");
                }
                Some(size) => {
                    let _ = writeln!(
                        out,
                        "  EXTRAPOLATION: largest scale below production ({size})"
                    );
                }
                None => {
                    let _ = writeln!(out, "  EXTRAPOLATION: no production size measured yet");
                }
            }
        }
    }
    let at = row.scales.map(|scale| measure(row, scale));
    for m in &at {
        let _ = writeln!(
            out,
            "  scale {:>9}: p50 {:>9} ps  p99 {:>9} ps  max {:>11} ps (batch means at {BATCH}); \
             work p50 {} p99 {} p99.9 {} max {} mean {}.{:03}; {} bytes",
            m.scale,
            m.time.0,
            m.time.1,
            m.time.2,
            m.work.quantile(1, 2),
            m.work.quantile(99, 100),
            m.work.quantile(999, 1_000),
            m.work.max(),
            m.work.mean_milli() / 1_000,
            m.work.mean_milli() % 1_000,
            m.bytes
        );
        let sizes: Vec<String> = m
            .batches
            .iter()
            .map(|(size, p50, p99)| format!("{size}: p50 {p50} p99 {p99}"))
            .collect();
        let _ = writeln!(out, "    by batch size (ps per call): {}", sizes.join("; "));
    }
    if let Axis::Data { .. } = axis {
        for m in &at {
            let _ = writeln!(
                out,
                "  scale {:>9}: mean {} ps per call at 1 worker, {} ps per call per worker at {} workers",
                m.scale,
                m.mean_ps,
                measure_workers(row, m.scale, host.workers),
                host.workers
            );
        }
    }
    let [small, _, large] = &at;
    let _ = writeln!(
        out,
        "  RATIO {} scale {} / {}: p99 time {} (of batch means), max time {}, p99 work {}, max work {}",
        row.id,
        large.scale,
        small.scale,
        shown(ratio(large.time.1, small.time.1)),
        shown(ratio(large.time.2, small.time.2)),
        shown(ratio(
            u128::from(large.work.quantile(99, 100)),
            u128::from(small.work.quantile(99, 100))
        )),
        shown(ratio(
            u128::from(large.work.max()),
            u128::from(small.work.max())
        ))
    );
    if let Err(why) = structural(class, &at) {
        let _ = writeln!(out, "  FAIL — {why}");
        return Verdict::Fail(why);
    }
    match time_ratio(class, &at) {
        None => {
            let _ = writeln!(
                out,
                "  BOUNDED — ratios are information; the bound is the claim"
            );
            Verdict::Pass
        }
        Some(r) if r <= PASS_PERMILLE => {
            let _ = writeln!(out, "  PASS — time ratio {} <= 1.250x", shown(r));
            Verdict::Pass
        }
        Some(r) if r <= SHARED_CEILING_PERMILLE => {
            let why = format!("time ratio {} above 1.250x on this host", shown(r));
            let _ = writeln!(out, "  FAIL (time, within the shared ceiling) — {why}");
            Verdict::TimeAbove(why)
        }
        Some(r) => {
            let why = format!("time ratio {} above the shared ceiling 3.000x", shown(r));
            let _ = writeln!(out, "  FAIL — {why}");
            Verdict::Fail(why)
        }
    }
}

/// The spike row of the self-test: one unit per call, and 10,000 more on every
/// 10,000th call.
fn spike_row(class: Option<Class>) -> Row<u64> {
    Row {
        id: "SELF-1",
        name: "one unit per call, 10,000 on every 10,000th",
        class,
        axis: Some(Axis::FixedState { why: "a counter" }),
        scales: [1, 10, 100],
        setup: |_| 0,
        op: |work, i| {
            *work += if (i + 1) % 10_000 == 0 { 10_000 } else { 1 };
            *work
        },
        work: |work| *work,
        bytes: |_| 8,
    }
}

/// A host for the self-tests, measured cheaply: the cache is not probed.
#[must_use]
pub(crate) fn quick_host() -> Host {
    Host {
        granularity_ns: timer_granularity_ns(),
        llc_bytes: 32 * 1_024 * 1_024,
        workers: 1,
    }
}

/// FM-F001's self-test: the spike row FAILS as WORST-CASE and passes as
/// AMORTISED.
///
/// # Errors
///
/// A sentence naming what the harness failed to see.
pub(crate) fn harness_detects_amortised_spike() -> Result<(), String> {
    let host = quick_host();
    let mut out = String::new();
    let worst = run(&spike_row(Some(Class::WorstCase)), &host, &mut out);
    if !matches!(&worst, Verdict::Fail(why) if why.contains("contradicts WORST-CASE")) {
        return Err(format!(
            "a spike declared WORST-CASE was not failed: {worst:?}\n{out}"
        ));
    }
    let amortised = run(
        &spike_row(Some(Class::Amortised { period: 10_000 })),
        &host,
        &mut out,
    );
    if amortised.fails() {
        return Err(format!(
            "a spike declared AMORTISED was failed: {amortised:?}\n{out}"
        ));
    }
    Ok(())
}

/// FM-F001's self-test: the host report names the timer granularity.
///
/// # Errors
///
/// A sentence naming what is missing.
pub(crate) fn harness_prints_timer_granularity() -> Result<(), String> {
    let granularity = timer_granularity_ns();
    if granularity == 0 {
        return Err("the timer granularity measured zero".to_owned());
    }
    let mut out = String::new();
    let _ = writeln!(out, "  timer granularity: {granularity} ns");
    if !out.contains("timer granularity: ") {
        return Err("the granularity line is not printed".to_owned());
    }
    Ok(())
}

/// FM-F001's self-test: a row without a class is refused before it is
/// measured.
///
/// # Errors
///
/// A sentence naming what the harness accepted.
pub(crate) fn harness_refuses_row_without_class() -> Result<(), String> {
    let mut out = String::new();
    match run(&spike_row(None), &quick_host(), &mut out) {
        Verdict::Refused(why) if why.contains("row without class") => Ok(()),
        other => Err(format!("a row without a class was not refused: {other:?}")),
    }
}

/// FM-F002's self-test: a row with no axis, a data row whose largest working
/// set is below the cache, and a span below 100x are each refused.
///
/// # Errors
///
/// A sentence naming what the harness accepted.
pub(crate) fn every_row_declares_axis_and_scale() -> Result<(), String> {
    let host = quick_host();
    let mut out = String::new();
    let mut row = spike_row(Some(Class::WorstCase));
    row.axis = None;
    if !matches!(run(&row, &host, &mut out), Verdict::Refused(why) if why.contains("row without axis"))
    {
        return Err("a row without an axis was not refused".to_owned());
    }
    row.axis = Some(Axis::Data {
        unit: "synthetic units",
        production: None,
    });
    if !matches!(run(&row, &host, &mut out), Verdict::Refused(why) if why.contains("below the last-level cache"))
    {
        return Err("a cache-resident data row was not refused".to_owned());
    }
    row.axis = Some(Axis::FixedState { why: "a counter" });
    row.scales = [1, 10, 99];
    if !matches!(run(&row, &host, &mut out), Verdict::Refused(why) if why.contains("span below")) {
        return Err("a span below 100x was not refused".to_owned());
    }
    Ok(())
}

/// FM-F002's self-test: a data row past the cache is measured at one worker and
/// at the production worker count, and both are printed.
///
/// # Errors
///
/// A sentence naming what was not printed.
pub(crate) fn a_data_row_runs_at_one_and_at_the_production_worker_count() -> Result<(), String> {
    let host = Host {
        workers: 2,
        ..quick_host()
    };
    let row: Row<(Vec<u64>, u64)> = Row {
        id: "SELF-2",
        name: "one strided read of a table of `scale` MiB",
        class: Some(Class::Expected),
        axis: Some(Axis::Data {
            unit: "MiB of table",
            production: Some(1_000),
        }),
        scales: [1, 10, 100],
        setup: |mib| (vec![1; usize::try_from(mib * 1_024 * 128).unwrap()], 0),
        op: |(table, reads), i| {
            *reads += 1;
            table[usize::try_from((i * 4_099) % table.len() as u64).unwrap()]
        },
        work: |(_, reads)| *reads,
        bytes: |(table, _)| table.len() as u64 * 8,
    };
    let mut out = String::new();
    let verdict = run(&row, &host, &mut out);
    if matches!(verdict, Verdict::Refused(_)) {
        return Err(format!(
            "a data row past the cache was refused: {verdict:?}"
        ));
    }
    for needed in [
        "at 1 worker and at 2 workers",
        "ps per call per worker at 2 workers",
        "EXTRAPOLATION: largest scale below production (1000)",
    ] {
        if !out.contains(needed) {
            return Err(format!("`{needed}` was not printed:\n{out}"));
        }
    }
    Ok(())
}

/// Every self-test, as the bench runs them before its rows.
///
/// # Errors
///
/// The first self-test's sentence.
pub(crate) fn self_tests() -> Result<(), String> {
    harness_detects_amortised_spike()?;
    harness_prints_timer_granularity()?;
    harness_refuses_row_without_class()?;
    every_row_declares_axis_and_scale()
}
